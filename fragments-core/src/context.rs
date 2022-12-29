use std::{
    any::{type_name, Any, TypeId},
    collections::HashMap,
};

use flax::{Component, ComponentValue};

/// Allows accessing a context value
pub struct ContextKey<T>(Component<T>);

impl<T> Clone for ContextKey<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Copy for ContextKey<T> {}

impl<T: ComponentValue> std::fmt::Debug for ContextKey<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ContextKey").field(&self.0).finish()
    }
}

impl<T: ComponentValue> ContextKey<T> {
    #[doc(hidden)]
    pub fn from_raw(component: Component<T>) -> Self {
        Self(component)
    }

    pub(crate) fn into_raw(self) -> Component<T> {
        self.0
    }

    pub fn name(&self) -> &'static str {
        self.0.name()
    }
}

/// Helper macro to declare a new statically typed context
#[macro_export]
macro_rules! context {
    ($($(#[$outer:meta])* $vis: vis $name: ident: $ty: ty,)*) => {
        $(
            $(#[$outer])*
            $vis fn $name() -> $crate::context::ContextKey<$ty> {
                flax::component! { $name: $ty, }
                $crate::context::ContextKey::from_raw($name())
            }
        )*
    };
}

#[cfg(test)]
mod test {
    use super::*;

    context! {
        pub foo: String,
        pub(crate) bar: i32,
    }

    #[test]
    fn context_key() {
        let foo = foo();
        let bar = bar();

        assert_ne!(foo.into_raw().key(), bar.into_raw().key());
        assert_eq!(foo.name(), "foo");
        assert_eq!(foo.into_raw().key(), self::foo().into_raw().key());
        assert_eq!(bar.name(), "bar");
    }
}

// pub trait ContextValue: Send + Sync {
//     fn type_name(&self) -> &'static str;
//     fn as_any(&self) -> &dyn Any;
//     fn as_any_mut(&mut self) -> &mut dyn Any;
// }

// impl<T> ContextValue for T
// where
//     T: Send + Sync,
// {
//     fn type_name(&self) -> &'static str {
//         type_name::<T>()
//     }

//     fn as_any(&self) -> &dyn Any {
//         self as &dyn Any
//     }

//     fn as_any_mut(&mut self) -> &mut dyn Any {
//         self as &mut dyn Any
//     }
// }

// impl dyn ContextValue {
//     pub fn downcast_ref<T>(&self) -> Option<&T> {
//         self.as_any().downcast_ref()
//     }

//     pub fn downcast_mut<T>(&mut self) -> Option<&mut T> {
//         self.as_any_mut().downcast_mut()
//     }
// }

// impl<T> ContextValue for T where T: Send + Sync {}

// pub(crate) struct ContextNode {
//     values: HashMap<TypeId, Box<dyn ContextValue>>,
// }

// impl ContextNode {
//     pub fn insert<T>(&mut self, value: T)
//     where
//         T: ContextValue,
//     {
//         self.values.insert(TypeId::of::<T>(), Box::new(value));
//     }

//     pub fn get<T>(&mut self) -> Option<&T> {
//         let value = self.values.get(&TypeId::of::<T>())?;
//         value.downcast_ref::<T>().unwrap()
//     }
// }
