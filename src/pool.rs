use std::cell::RefCell;
use std::fmt;
use std::sync::Mutex;

pub type CreateFn<T> = Box<Fn() -> T + Send>;

pub struct Pool<T> {
    stack: Mutex<RefCell<Vec<T>>>,
    create: CreateFn<T>,
}

impl<T> Pool<T> {
    pub fn new(create: CreateFn<T>) -> Pool<T> {
        Pool {
            stack: Mutex::new(RefCell::new(vec![])),
            create: create,
        }
    }

    pub fn get(&self) -> T {
        let stack = self.stack.lock();
        let stack = stack.unwrap();
        let mut stack = stack.borrow_mut();
        match stack.pop() {
            None => (self.create)(),
            Some(v) => v,
        }
    }

    pub fn put(&self, v: T) {
        let stack = self.stack.lock();
        let stack = stack.unwrap();
        stack.borrow_mut().push(v);
    }
}

impl<T: fmt::Debug> fmt::Debug for Pool<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let stack = self.stack.lock();
        let stack = stack.unwrap();
        stack.fmt(f)
    }
}
