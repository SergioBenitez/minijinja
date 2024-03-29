// use std::collections::BTreeSet;
// use std::fmt;
// use std::sync::Arc;
//
// use crate::error::{Error, ErrorKind};
// use crate::output::Output;
// use crate::utils::AutoEscape;
// use crate::value::{Enumeration, Object, Value, ValueRepr};
// use crate::vm::state::State;
// use crate::vm::Vm;
//
// #[derive(Debug)]
// pub(crate) struct Macro {
//     pub name: Arc<str>,
//     pub arg_spec: Vec<Arc<str>>,
//     // because values need to be 'static, we can't hold a reference to the
//     // instructions that declared the macro.  Instead of that we place the
//     // reference to the macro instruction (and the jump offset) in the
//     // state under `state.macros`.
//     pub macro_ref_id: usize,
//     pub state_id: isize,
//     pub closure: Value,
//     pub caller_reference: bool,
// }
//
// impl fmt::Display for Macro {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "<macro {}>", self.name)
//     }
// }
//
// impl Object for Macro {
//     fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
//         match key.as_str()? {
//             "name" => Some(Value::from(self.name.clone())),
//             "arguments" => Some(Value::from_object_iter(self.clone(), |this| {
//                 Box::new(this.arg_spec.iter().cloned().map(Value::from))
//             })),
//             "caller" => Some(Value::from(self.caller_reference)),
//             _ => None,
//         }
//     }
//
//     fn enumeration(self: &Arc<Self>) -> Enumeration {
//         Enumeration::Static(&["name", "arguments", "caller"])
//     }
//
//     fn call(
//         self: &Arc<Self>,
//         state: &State,
//         method: Option<&str>,
//         args: &[Value],
//     ) -> Result<Value, Error> {
//         if method.is_some() {
//             return Err(Error::new(
//                 ErrorKind::InvalidOperation,
//                 "cannot call method on macro",
//             ));
//         }
//
//         // we can only call macros that point to loaded template state.
//         if state.id != self.state_id {
//             return Err(Error::new(
//                 ErrorKind::InvalidOperation,
//                 "cannot call this macro. template state went away.",
//             ));
//         }
//
//         let (args, kwargs) = match args.last() {
//             Some(Value(ValueRepr::Object(obj))) => (&args[..args.len() - 1], Some(obj)),
//             _ => (args, None),
//         };
//
//         if args.len() > self.arg_spec.len() {
//             return Err(Error::from(ErrorKind::TooManyArguments));
//         }
//
//         let mut kwargs_used = BTreeSet::new();
//         let mut arg_values = Vec::with_capacity(self.arg_spec.len());
//         for (idx, name) in self.arg_spec.iter().enumerate() {
//             let kwarg = match kwargs {
//                 Some(kwargs) => kwargs.get_value(&Value::from(name.clone())),
//                 _ => None,
//             };
//             arg_values.push(match (args.get(idx), kwarg) {
//                 (Some(_), Some(_)) => {
//                     return Err(Error::new(
//                         ErrorKind::TooManyArguments,
//                         format!("duplicate argument `{name}`"),
//                     ))
//                 }
//                 (Some(arg), None) => arg.clone(),
//                 (None, Some(kwarg)) => {
//                     kwargs_used.insert(name as &str);
//                     kwarg.clone()
//                 }
//                 (None, None) => Value::UNDEFINED,
//             });
//         }
//
//         let caller = if self.caller_reference {
//             kwargs_used.insert("caller");
//             Some(
//                 kwargs
//                     .and_then(|x| x.get_value(&Value::from("caller")))
//                     .unwrap_or(Value::UNDEFINED),
//             )
//         } else {
//             None
//         };
//
//         if let Some(kwargs) = kwargs {
//             for key in kwargs.enumeration() {
//                 if let Some(name) = key.as_str() {
//                     if !kwargs_used.contains(name) {
//                         return Err(Error::new(
//                             ErrorKind::TooManyArguments,
//                             format!("unknown keyword argument `{key}`"),
//                         ));
//                     }
//                 }
//             }
//         }
//
//         let (instructions, offset) = &state.macros[self.macro_ref_id];
//         let vm = Vm::new(state.env());
//         let mut rv = String::new();
//         let mut out = Output::with_string(&mut rv);
//
//         // If a macro is self referential we need to put a reference to ourselves
//         // there.  Unfortunately because we only have a &self reference here, we
//         // cannot bump our own refcount.  Instead we need to wrap the macro data
//         // into an extra level of Arc to avoid unnecessary clones.
//         let closure = self.closure.clone();
//
//         // This requires some explanation here.  Because we get the state as &State and
//         // not &mut State we are required to create a new state here.  This is unfortunate
//         // but makes the calling interface more convenient for the rest of the system.
//         // Because macros cannot return anything other than strings (most importantly they)
//         // can't return other macros this is however not an issue, as modifications in the
//         // macro cannot leak out.
//         ok!(vm.eval_macro(
//             instructions,
//             *offset,
//             closure,
//             caller,
//             &mut out,
//             state,
//             arg_values
//         ));
//
//         Ok(if !matches!(state.auto_escape(), AutoEscape::None) {
//             Value::from_safe_string(rv)
//         } else {
//             Value::from(rv)
//         })
//     }
//
//     fn render(self: &Arc<Self>, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         fmt::Display::fmt(self, f)
//     }
// }

use std::collections::BTreeSet;
use std::fmt;
use std::sync::Arc;

use crate::error::{Error, ErrorKind};
use crate::output::Output;
use crate::utils::AutoEscape;
use crate::value::{Enumeration, Object, ObjectRepr, Value, ValueRepr};
use crate::vm::state::State;
use crate::vm::Vm;

pub(crate) struct Macro {
    pub name: Arc<str>,
    pub arg_spec: Vec<Arc<str>>,
    // because values need to be 'static, we can't hold a reference to the
    // instructions that declared the macro.  Instead of that we place the
    // reference to the macro instruction (and the jump offset) in the
    // state under `state.macros`.
    pub macro_ref_id: usize,
    pub state_id: isize,
    pub closure: Value,
    pub caller_reference: bool,
}

impl fmt::Debug for Macro {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<macro {}>", self.name)
    }
}

impl Object for Macro {
    fn repr(self: &Arc<Self>) -> ObjectRepr {
        ObjectRepr::Map
    }

    fn enumeration(self: &Arc<Self>) -> Enumeration {
        Enumeration::Static(&["name", "arguments", "caller"])
    }

    fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
        match key.as_str()? {
            "name" => Some(Value::from(self.name.clone())),
            "arguments" => Some(Value::from_object_iter(self.clone(), |this| {
                Box::new(this.arg_spec.iter().cloned().map(Value::from))
            })),
            "caller" => Some(Value::from(self.caller_reference)),
            _ => None,
        }
    }

    fn call(
        self: &Arc<Self>,
        state: &State<'_, '_>,
        method: Option<&str>,
        args: &[Value],
    ) -> Result<Value, Error> {
        if method.is_some() {
            return Err(Error::new(
                ErrorKind::InvalidOperation,
                "cannot call methods on macro",
            ));
        }

        // we can only call macros that point to loaded template state.
        if state.id != self.state_id {
            return Err(Error::new(
                ErrorKind::InvalidOperation,
                "cannot call this macro. template state went away.",
            ));
        }

        let (args, kwargs) = match args.last() {
            Some(Value(ValueRepr::Object(obj))) => match obj.as_kwargs() {
                Some(kwargs) => (&args[..args.len() - 1], Some(kwargs)),
                None => (args, None),
            },
            _ => (args, None),
        };

        if args.len() > self.arg_spec.len() {
            return Err(Error::from(ErrorKind::TooManyArguments));
        }

        let mut kwargs_used = BTreeSet::new();
        let mut arg_values = Vec::with_capacity(self.arg_spec.len());
        for (idx, name) in self.arg_spec.iter().enumerate() {
            let kwarg: Option<&Value> = match kwargs {
                Some(ref kwargs) => kwargs.get(name).ok(),
                _ => None,
            };
            arg_values.push(match (args.get(idx), kwarg) {
                (Some(_), Some(_)) => {
                    return Err(Error::new(
                        ErrorKind::TooManyArguments,
                        format!("duplicate argument `{name}`"),
                    ))
                }
                (Some(arg), None) => arg.clone(),
                (None, Some(kwarg)) => {
                    kwargs_used.insert(name as &str);
                    kwarg.clone()
                }
                (None, None) => Value::UNDEFINED,
            });
        }

        let caller = if self.caller_reference {
            kwargs_used.insert("caller");
            Some(
                kwargs
                    .as_ref()
                    .and_then(|x| x.get("caller").ok())
                    .unwrap_or(Value::UNDEFINED),
            )
        } else {
            None
        };

        if let Some(kwargs) = kwargs {
            for key in kwargs.values.keys().filter_map(|x| x.as_str()) {
                if !kwargs_used.contains(key) {
                    return Err(Error::new(
                        ErrorKind::TooManyArguments,
                        format!("unknown keyword argument `{key}`"),
                    ));
                }
            }
        }

        let (instructions, offset) = &state.macros[self.macro_ref_id];
        let vm = Vm::new(state.env());
        let mut rv = String::new();
        let mut out = Output::with_string(&mut rv);

        // If a macro is self referential we need to put a reference to ourselves
        // there.  Unfortunately because we only have a &self reference here, we
        // cannot bump our own refcount.  Instead we need to wrap the macro data
        // into an extra level of Arc to avoid unnecessary clones.
        let closure = self.closure.clone();

        // This requires some explanation here.  Because we get the state as &State and
        // not &mut State we are required to create a new state here.  This is unfortunate
        // but makes the calling interface more convenient for the rest of the system.
        // Because macros cannot return anything other than strings (most importantly they)
        // can't return other macros this is however not an issue, as modifications in the
        // macro cannot leak out.
        ok!(vm.eval_macro(
            instructions,
            *offset,
            closure,
            caller,
            &mut out,
            state,
            arg_values
        ));

        Ok(if !matches!(state.auto_escape(), AutoEscape::None) {
            Value::from_safe_string(rv)
        } else {
            Value::from(rv)
        })
    }

    fn render(self: &Arc<Self>, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<macro {}>", self.name)
    }
}
