//! Descripcion de la escena: que hay en el espacio y como se ve.
//!
//! [`relativity`] es la unica parte que contiene fisica de verdad; los demas
//! modulos la usan. Todos son funciones puras del punto y del tiempo: no saben
//! nada de pixeles, buffers ni camara. Mantenerlos puros es lo que permite
//! paralelizar por filas sin ningun tipo de bloqueo.
//!
//! No hay modulo de neblina. En la version anterior habia un aro de niebla
//! puesto a mano alrededor del horizonte para simular el halo. Con las geodesicas
//! bien integradas ese halo aparece solo, y es el anillo de fotones: la imagen
//! del disco dando vueltas alrededor del agujero antes de escapar. Un aro extra
//! solo taparia el fenomeno real con uno inventado.

pub mod blackhole;
pub mod disk;
pub mod relativity;
pub mod stars;
