//! Type mapping and resolution utilities for the nom-compiler bridge.

use std::collections::HashMap;

/// Classification of type kinds in the Nom type system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeKind {
    Primitive,
    Composite,
    Function,
    Generic,
    Unknown,
}

impl TypeKind {
    /// Returns `true` if the type is concrete (Primitive or Composite).
    pub fn is_concrete(&self) -> bool {
        matches!(self, TypeKind::Primitive | TypeKind::Composite)
    }

    /// Returns a short tag string representing the kind.
    pub fn kind_tag(&self) -> &'static str {
        match self {
            TypeKind::Primitive => "prim",
            TypeKind::Composite => "comp",
            TypeKind::Function => "fn",
            TypeKind::Generic => "gen",
            TypeKind::Unknown => "unk",
        }
    }
}

/// A newtype wrapper for type identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeId(u32);

impl TypeId {
    /// Returns `true` if the type ID represents a built-in type (ID < 100).
    pub fn is_builtin(&self) -> bool {
        self.0 < 100
    }

    /// Returns a formatted string key for the type ID.
    pub fn type_key(&self) -> String {
        format!("T{:04}", self.0)
    }
}

/// Information about a specific type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeInfo {
    pub id: TypeId,
    pub name: String,
    pub kind: TypeKind,
    pub size_bytes: u32,
}

impl TypeInfo {
    /// Returns `true` if the type is zero-sized (size_bytes == 0).
    pub fn is_zero_sized(&self) -> bool {
        self.size_bytes == 0
    }

    /// Returns a human-readable label for the type.
    pub fn label(&self) -> String {
        format!("{}::{} [{}b]", self.kind.kind_tag(), self.name, self.size_bytes)
    }
}

/// A map from type IDs to type information.
#[derive(Debug, Default, Clone)]
pub struct TypeMap {
    types: HashMap<u32, TypeInfo>,
}

impl TypeMap {
    /// Creates a new empty `TypeMap`.
    pub fn new() -> Self {
        Self {
            types: HashMap::new(),
        }
    }

    /// Inserts a type into the map.
    pub fn insert(&mut self, info: TypeInfo) {
        self.types.insert(info.id.0, info);
    }

    /// Returns the type information for the given ID, if present.
    pub fn get(&self, id: &TypeId) -> Option<&TypeInfo> {
        self.types.get(&id.0)
    }

    /// Returns all concrete types in the map, sorted by ID (ascending).
    pub fn concrete_types(&self) -> Vec<&TypeInfo> {
        let mut concrete: Vec<&TypeInfo> = self
            .types
            .values()
            .filter(|info| info.kind.is_concrete())
            .collect();
        concrete.sort_by_key(|info| info.id.0);
        concrete
    }

    /// Returns the total number of types in the map.
    pub fn type_count(&self) -> usize {
        self.types.len()
    }
}

/// Utility for resolving types by name and counting built-ins.
#[derive(Debug, Default, Clone)]
pub struct TypeResolver;

impl TypeResolver {
    /// Finds the first type in the map with the given name.
    pub fn resolve_by_name<'a>(map: &'a TypeMap, name: &str) -> Option<&'a TypeInfo> {
        map.types.values().find(|info| info.name == name)
    }

    /// Counts how many types in the map are built-in.
    pub fn builtin_count(map: &TypeMap) -> usize {
        map.types.values().filter(|info| info.id.is_builtin()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_kind_is_concrete() {
        assert!(TypeKind::Primitive.is_concrete());
        assert!(TypeKind::Composite.is_concrete());
        assert!(!TypeKind::Function.is_concrete());
        assert!(!TypeKind::Generic.is_concrete());
        assert!(!TypeKind::Unknown.is_concrete());
    }

    #[test]
    fn test_type_kind_kind_tag() {
        assert_eq!(TypeKind::Primitive.kind_tag(), "prim");
        assert_eq!(TypeKind::Composite.kind_tag(), "comp");
        assert_eq!(TypeKind::Function.kind_tag(), "fn");
        assert_eq!(TypeKind::Generic.kind_tag(), "gen");
        assert_eq!(TypeKind::Unknown.kind_tag(), "unk");
    }

    #[test]
    fn test_type_id_is_builtin_boundary() {
        assert!(TypeId(0).is_builtin());
        assert!(TypeId(99).is_builtin());
        assert!(!TypeId(100).is_builtin());
        assert!(!TypeId(101).is_builtin());
        assert!(!TypeId(1000).is_builtin());
    }

    #[test]
    fn test_type_id_type_key_format() {
        assert_eq!(TypeId(0).type_key(), "T0000");
        assert_eq!(TypeId(42).type_key(), "T0042");
        assert_eq!(TypeId(999).type_key(), "T0999");
        assert_eq!(TypeId(1000).type_key(), "T1000");
        assert_eq!(TypeId(12345).type_key(), "T12345");
    }

    #[test]
    fn test_type_info_is_zero_sized() {
        let zero_sized = TypeInfo {
            id: TypeId(1),
            name: "Unit".to_string(),
            kind: TypeKind::Primitive,
            size_bytes: 0,
        };
        assert!(zero_sized.is_zero_sized());

        let sized = TypeInfo {
            id: TypeId(2),
            name: "Int".to_string(),
            kind: TypeKind::Primitive,
            size_bytes: 4,
        };
        assert!(!sized.is_zero_sized());
    }

    #[test]
    fn test_type_info_label_format() {
        let info = TypeInfo {
            id: TypeId(42),
            name: "String".to_string(),
            kind: TypeKind::Composite,
            size_bytes: 24,
        };
        assert_eq!(info.label(), "comp::String [24b]");

        let primitive = TypeInfo {
            id: TypeId(1),
            name: "bool".to_string(),
            kind: TypeKind::Primitive,
            size_bytes: 1,
        };
        assert_eq!(primitive.label(), "prim::bool [1b]");
    }

    #[test]
    fn test_type_map_insert_and_get() {
        let mut map = TypeMap::new();
        let info = TypeInfo {
            id: TypeId(100),
            name: "MyType".to_string(),
            kind: TypeKind::Composite,
            size_bytes: 8,
        };

        map.insert(info.clone());
        assert_eq!(map.type_count(), 1);
        
        let retrieved = map.get(&TypeId(100));
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "MyType");
        
        assert!(map.get(&TypeId(999)).is_none());
    }

    #[test]
    fn test_type_map_concrete_types_sorted() {
        let mut map = TypeMap::new();
        
        // Add some types in non-sorted order
        map.insert(TypeInfo {
            id: TypeId(300),
            name: "C".to_string(),
            kind: TypeKind::Composite,
            size_bytes: 16,
        });
        
        map.insert(TypeInfo {
            id: TypeId(100),
            name: "A".to_string(),
            kind: TypeKind::Primitive,
            size_bytes: 4,
        });
        
        // This one is not concrete, should be filtered out
        map.insert(TypeInfo {
            id: TypeId(200),
            name: "Func".to_string(),
            kind: TypeKind::Function,
            size_bytes: 0,
        });
        
        map.insert(TypeInfo {
            id: TypeId(150),
            name: "B".to_string(),
            kind: TypeKind::Composite,
            size_bytes: 8,
        });
        
        let concrete = map.concrete_types();
        assert_eq!(concrete.len(), 3); // Only A, B, C (not Func)
        assert_eq!(concrete[0].id.0, 100); // A
        assert_eq!(concrete[1].id.0, 150); // B
        assert_eq!(concrete[2].id.0, 300); // C
    }

    #[test]
    fn test_type_resolver_builtin_count() {
        let mut map = TypeMap::new();
        
        // Add built-in types (ID < 100)
        map.insert(TypeInfo {
            id: TypeId(1),
            name: "int".to_string(),
            kind: TypeKind::Primitive,
            size_bytes: 4,
        });
        
        map.insert(TypeInfo {
            id: TypeId(2),
            name: "bool".to_string(),
            kind: TypeKind::Primitive,
            size_bytes: 1,
        });
        
        map.insert(TypeInfo {
            id: TypeId(99),
            name: "string".to_string(),
            kind: TypeKind::Composite,
            size_bytes: 24,
        });
        
        // Add non-built-in types
        map.insert(TypeInfo {
            id: TypeId(100),
            name: "UserType".to_string(),
            kind: TypeKind::Composite,
            size_bytes: 32,
        });
        
        map.insert(TypeInfo {
            id: TypeId(101),
            name: "Another".to_string(),
            kind: TypeKind::Generic,
            size_bytes: 0,
        });
        
        assert_eq!(TypeResolver::builtin_count(&map), 3);
    }
}