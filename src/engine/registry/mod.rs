//! Generic registry primitives.
//!
//! Purpose:
//! Store definitions behind stable numeric IDs and human-readable keys.
//!
//! Inputs:
//! Definition values that expose a unique string key.
//!
//! Outputs:
//! Stable IDs and immutable definition lookup.
//!
//! Extension points:
//! Future data loading can deserialize definitions and register them through
//! the same API used by hand-written bootstrap content.

use std::collections::HashMap;
use std::fmt;

/// Trait implemented by registry definitions.
pub trait Definition {
    /// Stable content key, for example `humancraft:stone`.
    fn key(&self) -> &str;
}

/// Error returned when a duplicate key is registered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryError {
    key: String,
}

impl RegistryError {
    pub fn duplicate_key(key: impl Into<String>) -> Self {
        Self { key: key.into() }
    }
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "definition key already registered: {}", self.key)
    }
}

impl std::error::Error for RegistryError {}

/// Stores definitions and assigns compact stable IDs.
#[derive(Debug, Clone)]
pub struct Registry<Id, T> {
    entries: Vec<T>,
    keys: HashMap<String, Id>,
}

impl<Id, T> Default for Registry<Id, T> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            keys: HashMap::new(),
        }
    }
}

impl<Id, T> Registry<Id, T>
where
    Id: Copy + From<usize> + Into<usize>,
    T: Definition,
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, definition: T) -> Result<Id, RegistryError> {
        let key = definition.key().to_owned();
        if self.keys.contains_key(&key) {
            return Err(RegistryError::duplicate_key(key));
        }

        let id = Id::from(self.entries.len());
        self.entries.push(definition);
        self.keys.insert(key, id);
        Ok(id)
    }

    pub fn get(&self, id: Id) -> Option<&T> {
        self.entries.get(id.into())
    }

    pub fn get_by_key(&self, key: &str) -> Option<(Id, &T)> {
        self.keys
            .get(key)
            .copied()
            .and_then(|id| self.get(id).map(|definition| (id, definition)))
    }

    pub fn id_for_key(&self, key: &str) -> Option<Id> {
        self.keys.get(key).copied()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (Id, &T)> {
        self.entries
            .iter()
            .enumerate()
            .map(|(index, definition)| (Id::from(index), definition))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestDefinition {
        key: String,
    }

    impl Definition for TestDefinition {
        fn key(&self) -> &str {
            &self.key
        }
    }

    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    struct TestId(usize);

    impl From<usize> for TestId {
        fn from(value: usize) -> Self {
            Self(value)
        }
    }

    impl From<TestId> for usize {
        fn from(value: TestId) -> Self {
            value.0
        }
    }

    #[test]
    fn assigns_stable_ids_in_registration_order() {
        let mut registry = Registry::<TestId, TestDefinition>::new();

        let first = registry
            .register(TestDefinition {
                key: "test:first".to_string(),
            })
            .unwrap();
        let second = registry
            .register(TestDefinition {
                key: "test:second".to_string(),
            })
            .unwrap();

        assert_eq!(first, TestId(0));
        assert_eq!(second, TestId(1));
        assert_eq!(registry.id_for_key("test:first"), Some(first));
    }

    #[test]
    fn rejects_duplicate_keys() {
        let mut registry = Registry::<TestId, TestDefinition>::new();
        registry
            .register(TestDefinition {
                key: "test:first".to_string(),
            })
            .unwrap();

        let result = registry.register(TestDefinition {
            key: "test:first".to_string(),
        });

        assert!(result.is_err());
    }
}
