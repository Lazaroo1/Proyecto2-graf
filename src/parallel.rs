//! Trabajo por bloques con hilos persistentes de la biblioteca estandar.
//!
//! `chunks_mut` entrega regiones disjuntas: ningun pixel tiene dos escritores.
//! La cola solo se bloquea para retirar un bloque, nunca durante su calculo.
//! El reparto dinamico equilibra las filas vacias y las que atraviesan el gas.
//! Reutilizar los hilos evita pagar su creacion en cada pasada de cada frame.

use std::any::Any;
use std::cell::Cell;
use std::panic::{self, AssertUnwindSafe};
use std::sync::{mpsc, Arc, Condvar, Mutex, OnceLock};
use std::thread;

type Panic = Box<dyn Any + Send>;
type Job = Box<dyn FnOnce() + Send + 'static>;

struct Pool {
    sender: mpsc::Sender<Job>,
    workers: usize,
}

impl Pool {
    fn shared() -> &'static Self {
        static POOL: OnceLock<Pool> = OnceLock::new();
        POOL.get_or_init(|| {
            let available = thread::available_parallelism().map_or(1, |n| n.get());
            let (sender, receiver) = mpsc::channel::<Job>();
            let receiver = Arc::new(Mutex::new(receiver));
            // El hilo llamador es uno de los trabajadores.
            let mut workers = 1;
            for index in 1..available {
                let receiver = Arc::clone(&receiver);
                let spawned = thread::Builder::new()
                    .name(format!("render-{index}"))
                    .spawn(move || loop {
                        let next = receiver.lock().unwrap_or_else(|e| e.into_inner()).recv();
                        match next {
                            Ok(job) => job(),
                            Err(_) => break,
                        }
                    });
                if spawned.is_err() {
                    break;
                }
                workers += 1;
            }
            Pool { sender, workers }
        })
    }

    /// # Safety
    /// El llamador debe esperar a que el job termine antes de destruir sus
    /// prestamos, tambien si hay panic. Solo `run_chunks` usa este metodo.
    unsafe fn submit_borrowed(&self, job: Box<dyn FnOnce() + Send + '_>) {
        // SAFETY: el contrato exige que los datos prestados sobrevivan al job.
        // WaitForBatch garantiza esa espera incluso durante el unwinding.
        let job: Job = unsafe { std::mem::transmute(job) };
        if let Err(error) = self.sender.send(job) {
            // Si la cola no esta disponible, completar el trabajo en este hilo.
            error.0();
        }
    }
}

#[derive(Default)]
struct BatchState {
    pending: usize,
    panic: Option<Panic>,
}

#[derive(Default)]
struct Batch {
    state: Mutex<BatchState>,
    finished: Condvar,
}

impl Batch {
    fn ticket(self: &Arc<Self>) -> Ticket {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).pending += 1;
        Ticket(Arc::clone(self))
    }

    fn wait(&self) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        while state.pending != 0 {
            state = self.finished.wait(state).unwrap_or_else(|e| e.into_inner());
        }
    }

    fn run(&self, work: &impl Fn()) {
        if let Err(error) = panic::catch_unwind(AssertUnwindSafe(work)) {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            if state.panic.is_none() {
                state.panic = Some(error);
            }
        }
    }
}

// Un ticket se libera tanto al completar como al descartar un job no ejecutado.
struct Ticket(Arc<Batch>);
impl Drop for Ticket {
    fn drop(&mut self) {
        let mut state = self.0.state.lock().unwrap_or_else(|e| e.into_inner());
        state.pending -= 1;
        if state.pending == 0 {
            self.0.finished.notify_all();
        }
    }
}

struct WaitForBatch<'a>(&'a Batch);
impl Drop for WaitForBatch<'_> {
    fn drop(&mut self) {
        self.0.wait();
    }
}

thread_local! {
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
}

struct ActiveWork(bool);
impl ActiveWork {
    fn enter() -> Self {
        Self(ACTIVE.replace(true))
    }
}
impl Drop for ActiveWork {
    fn drop(&mut self) {
        ACTIVE.set(self.0);
    }
}

/// Procesa bloques con su indice, esperando todos los hilos antes de volver.
/// `min_per_worker` evita despachar jobs para los niveles pequenos del bloom.
pub fn chunks_mut<T: Send>(
    data: &mut [T],
    chunk_len: usize,
    min_per_worker: usize,
    operation: impl Fn(usize, &mut [T]) + Sync,
) {
    let available = Pool::shared().workers;
    let workers = available.min(data.len() / min_per_worker.max(1)).max(1);
    run_chunks(data, chunk_len, workers, operation);
}

fn run_chunks<T: Send>(
    data: &mut [T],
    chunk_len: usize,
    workers: usize,
    operation: impl Fn(usize, &mut [T]) + Sync,
) {
    assert!(chunk_len > 0, "el tamano de bloque debe ser positivo");
    let workers = workers.min(data.len().div_ceil(chunk_len));
    // Un callback anidado no espera al mismo pool que ya esta ocupando.
    if workers <= 1 || ACTIVE.get() {
        for (index, chunk) in data.chunks_mut(chunk_len).enumerate() {
            operation(index, chunk);
        }
        return;
    }

    let jobs = Mutex::new(data.chunks_mut(chunk_len).enumerate());
    let work = || {
        let _active = ActiveWork::enter();
        loop {
            // El guard se libera antes de ejecutar la operacion del usuario.
            let next = jobs.lock().unwrap_or_else(|e| e.into_inner()).next();
            let Some((index, chunk)) = next else { break };
            operation(index, chunk);
        }
    };
    let batch = Arc::new(Batch::default());
    // Debe declararse DESPUES de `work` y `jobs`: se destruye primero y espera
    // todos los prestamos antes de que desaparezca cualquiera de esos datos.
    let _wait = WaitForBatch(&batch);
    for _ in 1..workers.min(Pool::shared().workers) {
        let work = &work;
        let ticket = batch.ticket();
        let job: Box<dyn FnOnce() + Send + '_> = Box::new(move || {
            ticket.0.run(work);
            // Despues de `work`, no se vuelve a acceder a ningun dato prestado.
            drop(ticket);
        });
        // SAFETY: _wait vive hasta el final de este bloque y espera todos los
        // tickets. Las excepciones del callback se capturan; incluso si enviar
        // un job falla o este hilo panica, los prestamos siguen vivos al esperar.
        unsafe { Pool::shared().submit_borrowed(job) };
    }
    batch.run(&work);
    batch.wait();
    let error = batch
        .state
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .panic
        .take();
    if let Some(error) = error {
        panic::resume_unwind(error);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn borrowed_rows_match_serial_processing_including_partial_last_row() {
        let source: Vec<usize> = (0..103).map(|i| i * i).collect();
        for workers in [1, 2, 7, 200] {
            let mut output = vec![0; source.len()];
            run_chunks(&mut output, 7, workers, |row_index, row| {
                for (x, value) in row.iter_mut().enumerate() {
                    *value += source[row_index * 7 + x] + 1;
                }
            });
            assert_eq!(
                output,
                source.iter().map(|value| value + 1).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn empty_work_and_worker_panics_are_not_silently_ignored() {
        run_chunks::<u8>(&mut [], 1, 4, |_, _| panic!("trabajo vacio"));
        let result = std::panic::catch_unwind(|| {
            let mut values = [0; 5];
            run_chunks(&mut values, 1, 3, |index, value| {
                assert_ne!(index, 2, "fallo de prueba");
                value[0] = 1;
            });
        });
        assert!(result.is_err());
    }

    #[test]
    fn nested_work_completes_without_waiting_on_its_own_pool() {
        let mut values = [0; 27];
        run_chunks(&mut values, 5, 3, |_, row| {
            chunks_mut(row, 2, 1, |_, pair| {
                for value in pair {
                    *value += 1;
                }
            });
        });
        assert_eq!(values, [1; 27]);
    }

    #[test]
    fn panics_wait_for_other_borrowed_work_before_returning() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Barrier;
        if Pool::shared().workers < 2 {
            return;
        }
        let barrier = Barrier::new(2);
        let completed = AtomicBool::new(false);
        let result = panic::catch_unwind(AssertUnwindSafe(|| {
            run_chunks(&mut [0; 2], 1, 2, |index, _| {
                barrier.wait();
                if index == 0 {
                    panic!("fallo mientras otro hilo conserva un prestamo");
                }
                thread::sleep(std::time::Duration::from_millis(5));
                completed.store(true, Ordering::SeqCst);
            });
        }));
        assert!(result.is_err());
        assert!(completed.load(Ordering::SeqCst));
        let mut next_frame = [0; 33];
        run_chunks(&mut next_frame, 4, 2, |_, row| row.fill(7));
        assert_eq!(next_frame, [7; 33]);
    }
}
