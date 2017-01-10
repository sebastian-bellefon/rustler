use ::{NifDecoder, NifEnv, NifResult};
use ::wrapper::nif_interface::{NIF_TERM, enif_snprintf};
use std::fmt::{self, Debug};
use std::os::raw::c_char;

/// NifTerm is used to represent all erlang terms. Terms are always lifetime limited by a NifEnv.
///
/// NifTerm is cloneable and copyable, but it can not exist outside of the lifetime of the NifEnv
/// that owns it.
#[derive(Clone, Copy)]
pub struct NifTerm<'a> {
    term: NIF_TERM,
    env: NifEnv<'a>,
}

impl<'a> Debug for NifTerm<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        const SIZE: usize = 1024;
        let mut bytes: Vec<u8> = Vec::with_capacity(SIZE);

        let mut n = 0;
        for _ in 0 .. 10 {
            let i = unsafe {
                enif_snprintf(bytes.as_mut_ptr() as *mut c_char,
                              bytes.capacity(),
                              b"%T\x00" as *const u8 as *const c_char,
                              self.as_c_arg())
            };
            if i < 0 {
                // Do not propagate an error, because string formatting is
                // supposed to be infallible.
                break;
            }

            n = i as usize;
            if n >= bytes.capacity() {
                // Bizarrely, enif_snprintf consistently underestimates the
                // amount of memory it will need to write long lists. To try to
                // avoid going around the loop again, double the estimate.
                bytes.reserve_exact(2 * n + 1);
            } else {
                break;
            }
        }

        unsafe {
            bytes.set_len(n);
        }
        f.write_str(&String::from_utf8_lossy(&bytes))
    }
}

impl<'a> NifTerm<'a> {
    pub fn new(env: NifEnv<'a>, inner: NIF_TERM) -> Self {
        NifTerm {
            term: inner,
            env: env,
        }
    }
    /// This extracts the raw term pointer. It is usually used in order to obtain a type that can
    /// be passed to calls into the erlang vm.
    pub fn as_c_arg(&self) -> NIF_TERM {
        self.term
    }

    pub fn get_env(&self) -> NifEnv<'a> {
        self.env
    }

    /// This will coerce the NifTerm into the given NifEnv without providing any checks
    /// or other operations. This is unsafe as it allows you to make a NifTerm usable
    /// on an env other then the one that owns it.
    ///
    /// The one case where this is acceptable to use is when we already know the NifEnv
    /// is the same, but we need to coerce the lifetime.
    pub unsafe fn env_cast<'b>(&self, env: NifEnv<'b>) -> NifTerm<'b> {
        NifTerm::new(env, self.as_c_arg())
    }

    /// Returns a representation of self in the given NifEnv.
    ///
    /// If the term is already is in the provided env, it will be directly returned. Otherwise
    /// the term will be copied over.
    pub fn in_env<'b>(&self, env: NifEnv<'b>) -> NifTerm<'b> {
        if self.get_env() == env {
            unsafe { self.env_cast(env) }
        } else {
            NifTerm::new(env, unsafe { ::wrapper::copy_term(env.as_c_arg(), self.as_c_arg()) })
        }
    }

    /// Decodes the NifTerm into type T.
    ///
    /// This should be used as the primary method of extracting the value from a NifTerm.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let term: NifTerm = ...;
    /// let number: i32 = try!(term.decode());
    /// ```
    pub fn decode<T>(self) -> NifResult<T> where T: NifDecoder<'a> {
        NifDecoder::decode(self)
    }
}
