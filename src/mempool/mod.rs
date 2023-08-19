use std::{ops::{Deref, DerefMut}, mem::MaybeUninit};

pub mod caching;

pub trait MemoryProvider<T> {

    type Object: Deref<Target = [T]> + DerefMut;

    unsafe fn get_new<F: FnOnce(&mut [MaybeUninit<T>])>(&self, size: usize, initializer: F) -> Self::Object;

    fn get_new_init<F: FnMut(usize) -> T>(&self, size: usize, mut initializer: F) -> Self::Object {
        unsafe {
            self.get_new(size, |mem| {
                for i in 0..mem.len() {
                    mem[i] = MaybeUninit::new(initializer(i))
                }
            })
        }
    }

    fn try_get_new_init<E, F: FnMut(usize) -> Result<T, E>>(&self, size: usize, mut initializer: F) -> Result<Self::Object, E> {
        unsafe {
            let mut aborted = None;
            let result = self.get_new(size, |mem| {
                let mut i = 0;
                while i < mem.len() {
                    // note that this will leak memory if initializer(i) panics
                    match initializer(i) {
                        Ok(val) => {
                            mem[i] = MaybeUninit::new(val);
                            i += 1;
                        },
                        Err(err) => {
                            aborted = Some(err);
                            // drop the previously initialized memory
                            // note that this does not prevent a memory leak in the panic case
                            for j in 0..i {
                                mem[j].assume_init_drop();
                            }
                            break;
                        }
                    };
                }
            });
            if let Some(err) = aborted {
                Err(err)
            } else {
                Ok(result)
            }
        }
    }
}

pub trait GrowableMemoryProvider<T>: MemoryProvider<T> {

    unsafe fn grow<F: FnOnce(&mut [MaybeUninit<T>])>(&self, el: &mut Self::Object, new_size: usize, initializer: F);

    fn grow_init<F: FnMut(usize) -> T>(&self, el: &mut Self::Object, new_size: usize, mut initializer: F) {
        assert!(new_size > el.len());
        let old_len = el.len();
        unsafe {
            self.grow(el, new_size, |mem| {
                for i in 0..mem.len() {
                    mem[i] = MaybeUninit::new(initializer(old_len + i))
                }
            })
        }
    }

}

#[derive(Copy, Clone)]
pub struct AllocatingMemoryProvider;

impl<T> MemoryProvider<T> for AllocatingMemoryProvider {
    
    type Object = Vec<T>;

    unsafe fn get_new<F: FnOnce(&mut [MaybeUninit<T>])>(&self, size: usize, initializer: F) -> Self::Object {
        let mut result = Box::new_uninit_slice(size);
        initializer(&mut *result);
        return result.assume_init().into_vec();
    }
}

impl<T> GrowableMemoryProvider<T> for AllocatingMemoryProvider {
    
    unsafe fn grow<F: FnOnce(&mut [MaybeUninit<T>])>(&self, el: &mut Vec<T>, new_size: usize, initializer: F) {
        assert!(new_size > el.len());
        let old_len = el.len();
        el.reserve(new_size - old_len);
        initializer(&mut el.spare_capacity_mut()[..(new_size - old_len)]);
        el.set_len(new_size);
    }
}

impl Default for AllocatingMemoryProvider {

    fn default() -> Self {
        AllocatingMemoryProvider
    }
}

#[derive(Clone)]
pub struct LoggingMemoryProvider {
    description: String
}

impl LoggingMemoryProvider {

    pub fn new(description: String) -> Self {
        LoggingMemoryProvider { description }
    }
}

impl<T> MemoryProvider<T> for LoggingMemoryProvider {
    
    type Object = Vec<T>;

    unsafe fn get_new<F: FnOnce(&mut [MaybeUninit<T>])>(&self, size: usize, initializer: F) -> Self::Object {
        println!("[{}]: Allocating {} entries", self.description, size);
        AllocatingMemoryProvider.get_new(size, initializer)
    }
}

impl<T> GrowableMemoryProvider<T> for LoggingMemoryProvider {
    
    unsafe fn grow<F: FnOnce(&mut [MaybeUninit<T>])>(&self, el: &mut Vec<T>, new_size: usize, initializer: F) {
        assert!(new_size > el.len());
        let old_len = el.len();
        el.reserve(new_size - old_len);
        initializer(&mut el.spare_capacity_mut()[..(new_size - old_len)]);
        el.set_len(new_size);
    }
}