use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, Arc};

use crate::error::{Error, ErrorKind};
use crate::value::{Object, MapObject, Value};
use crate::vm::state::State;

// TODO: Remove this wrapper once everything gives `&Arc<Self>`.
#[derive(Clone)]
pub(crate) struct Loop {
    pub status: Arc<LoopStatus>,
}

impl std::ops::Deref for Loop {
    type Target = LoopStatus;

    fn deref(&self) -> &Self::Target {
        &*self.status
    }
}

pub(crate) struct LoopStatus {
    pub len: usize,
    pub idx: AtomicUsize,
    pub depth: usize,
    #[cfg(feature = "adjacent_loop_items")]
    pub value_triple: Mutex<(Option<Value>, Option<Value>, Option<Value>)>,
    pub last_changed_value: Mutex<Option<Vec<Value>>>,
}

impl fmt::Debug for Loop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = f.debug_struct("Loop");
        for attr in self.static_fields().unwrap() {
            s.field(attr, &self.get_field(&Value::from(*attr)).unwrap());
        }
        s.finish()
    }
}

impl Object for Loop {
    fn value(self: &Arc<Self>) -> Value {
        Value::from_any_map_object(self.clone())
    }

    fn call(self: &Arc<Self>, _state: &State, _args: &[Value]) -> Result<Value, Error> {
        Err(Error::new(
            ErrorKind::InvalidOperation,
            "loop cannot be called if reassigned to different variable",
        ))
    }

    fn call_method(self: &Arc<Self>, _state: &State, name: &str, args: &[Value]) -> Result<Value, Error> {
        if name == "changed" {
            let mut last_changed_value = self.last_changed_value.lock().unwrap();
            let value = args.to_owned();
            let changed = last_changed_value.as_ref() != Some(&value);
            if changed {
                *last_changed_value = Some(value);
                Ok(Value::from(true))
            } else {
                Ok(Value::from(false))
            }
        } else if name == "cycle" {
            let idx = self.idx.load(Ordering::Relaxed);
            match args.get(idx % args.len()) {
                Some(arg) => Ok(arg.clone()),
                None => Ok(Value::UNDEFINED),
            }
        } else {
            Err(Error::new(
                ErrorKind::UnknownMethod,
                format!("loop object has no method named {name}"),
            ))
        }
    }
}

impl MapObject for Loop {
    fn static_fields(&self) -> Option<&'static [&'static str]> {
        Loop::static_fields(&self)
    }

    fn get_field(self: &Arc<Self>, key: &Value) -> Option<Value> {
        Loop::get_field(&self, key)
    }
}

impl Loop {
    fn static_fields(&self) -> Option<&'static [&'static str]> {
        Some(
            &[
                "index0",
                "index",
                "length",
                "revindex",
                "revindex0",
                "first",
                "last",
                "depth",
                "depth0",
                #[cfg(feature = "adjacent_loop_items")]
                "previtem",
                #[cfg(feature = "adjacent_loop_items")]
                "nextitem",
            ][..],
        )
    }

    fn get_field(&self, key: &Value) -> Option<Value> {
        let name = key.as_str()?;
        let idx = self.idx.load(Ordering::Relaxed) as u64;
        // if we never iterated, then all attributes are undefined.
        // this can happen in some rare circumstances where the engine
        // did not manage to iterate
        if idx == !0 {
            return Some(Value::UNDEFINED);
        }
        let len = self.len as u64;
        match name {
            "index0" => Some(Value::from(idx)),
            "index" => Some(Value::from(idx + 1)),
            "length" => Some(Value::from(len)),
            "revindex" => Some(Value::from(len.saturating_sub(idx))),
            "revindex0" => Some(Value::from(len.saturating_sub(idx).saturating_sub(1))),
            "first" => Some(Value::from(idx == 0)),
            "last" => Some(Value::from(len == 0 || idx == len - 1)),
            "depth" => Some(Value::from(self.depth + 1)),
            "depth0" => Some(Value::from(self.depth)),
            #[cfg(feature = "adjacent_loop_items")]
            "previtem" => Some(
                self.value_triple
                    .lock()
                    .unwrap()
                    .0
                    .clone()
                    .unwrap_or(Value::UNDEFINED),
            ),
            #[cfg(feature = "adjacent_loop_items")]
            "nextitem" => Some(
                self.value_triple
                    .lock()
                    .unwrap()
                    .2
                    .clone()
                    .unwrap_or(Value::UNDEFINED),
            ),
            _ => None,
        }
    }
}

impl fmt::Display for Loop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "<loop {}/{}>",
            self.idx.load(Ordering::Relaxed),
            self.len
        )
    }
}
