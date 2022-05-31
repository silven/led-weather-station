use std::error::Error;
use std::sync::{Arc, Mutex};

pub struct Mailbox<T> {
    inner: Arc<MailboxInner<T>>,
}

struct MailboxInner<T> {
    value: Mutex<Option<T>>,
}

impl<T> Mailbox<T> {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(MailboxInner {
                value: Mutex::new(None),
            }),
        }
    }

    pub fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }

    pub fn put(&self, value: T) -> Result<(), Box<dyn Error + '_>> {
        let mut x = self.inner.value.lock()?;
        *x = Some(value);
        Ok(())
    }

    pub fn if_new(&self, mut callback: impl FnMut(T)) -> Result<(), Box<dyn Error + '_>> {
        let mut x = self.inner.value.lock()?;
        if let Some(v) = x.take() {
            callback(v);
        }
        Ok(())
    }
}
