#![forbid(unsafe_code)]

use std::{cell::RefCell, collections::VecDeque, fmt::Debug, rc::Rc};

use thiserror::Error;
use uuid::Uuid;

////////////////////////////////////////////////////////////////////////////////

#[derive(Error, Debug)]
#[error("channel is closed")]
pub struct SendError<T: Debug> {
    pub value: T,
}

pub struct Sender<T> {
    buf: Rc<RefCell<MPSCBuffer<T>>>,
    closed: bool,
    id: Uuid,
}

impl<T: Debug> Sender<T> {
    pub fn send(&self, value: T) -> Result<(), SendError<T>> {
        let mut buf = self.buf.as_ref().borrow_mut();
        if self.closed || buf.closed {
            return Err(SendError { value });
        }
        buf.buf.push_back(value);
        Ok(())
    }

    pub fn is_closed(&self) -> bool {
        let buf = self.buf.as_ref().borrow();
        self.closed || buf.closed
    }

    pub fn same_channel(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Self {
            buf: self.buf.clone(),
            closed: self.closed.clone(),
            id: self.id.clone(),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Error, Debug)]
pub enum ReceiveError {
    #[error("channel is empty")]
    Empty,
    #[error("channel is closed")]
    Closed,
}

pub struct Receiver<T> {
    buf: Rc<RefCell<MPSCBuffer<T>>>,
}

impl<T> Receiver<T> {
    pub fn recv(&mut self) -> Result<T, ReceiveError> {
        let mut buf = self.buf.as_ref().borrow_mut();
        match buf.buf.pop_front() {
            None => {
                return if Rc::strong_count(&self.buf) == 1 || buf.closed {
                    Err(ReceiveError::Closed)
                } else {
                    Err(ReceiveError::Empty)
                }
            }
            Some(e) => Ok(e),
        }
    }

    pub fn close(&mut self) {
        let mut buf = self.buf.as_ref().borrow_mut();
        buf.closed = true;
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        self.close()
    }
}

struct MPSCBuffer<T> {
    buf: VecDeque<T>,
    closed: bool,
}

////////////////////////////////////////////////////////////////////////////////

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let buffer = Rc::new(RefCell::new(MPSCBuffer {
        buf: VecDeque::new(),
        closed: false,
    }));
    (
        Sender {
            buf: buffer.clone(),
            closed: false,
            id: Uuid::new_v4(),
        },
        Receiver {
            buf: buffer.clone(),
        },
    )
}
