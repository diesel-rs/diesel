use std::any::{TypeId, Any};
use std::collections::{BTreeMap, btree_map};
use std::fmt;
use std::path::Path;

use proc_macro::TokenStream;
use connection::SimpleConnection;
use super::{RunMigrationsError, MigrationError, Migration};


// Temporary until this method is stabilised on `Any`
#[doc(hidden)]
pub unsafe trait AnyType: Any {
    fn type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}
unsafe impl<T: Any> AnyType for T {}

/// Trait implemented by types representing metadata about a migration
pub trait MigrationAnnotation: AnyType + fmt::Debug {
    #[doc(hidden)]
    fn embed(&self) -> Result<TokenStream, MigrationError> {
        Err(MigrationError::MigrationNotEmbeddable)
    }
}

impl MigrationAnnotation {
    // Allow downcasting to a specific kind of annotation
    fn downcast_ref<T: MigrationAnnotation>(&self) -> Option<&T> {
        unsafe {
            if self.type_id() == TypeId::of::<T>() {
                Some(&*(self as *const MigrationAnnotation as *const T))
            } else {
                None
            }
        }
    }
}

/// A migration with some annotations attached
#[derive(Debug)]
pub struct AnnotatedMigration {
    migration: Box<Migration>,
    annotations: BTreeMap<TypeId, Box<MigrationAnnotation>>,
}

impl AnnotatedMigration {
    /// Create from a migration without any annotations
    pub fn new(migration: Box<Migration>) -> Self {
        AnnotatedMigration {
            migration,
            annotations: BTreeMap::new()
        }
    }
    /// Look for an annotation of a known type
    pub fn annotation<T: MigrationAnnotation>(&self) -> Option<&T> {
        self.annotations.get(&TypeId::of::<T>()).map(|annotation| {
            annotation.downcast_ref().expect("MigrationAnnotation stored under incorrect key")
        })
    }
    /// Iterate over all annotations
    pub fn annotations(&self) -> MigrationAnnotationsIter {
        MigrationAnnotationsIter(self.annotations.values())
    }
    /// Add a new annotation
    pub fn annotate<T: MigrationAnnotation>(&mut self, annotation: T) {
        self.annotations.insert(TypeId::of::<T>(), Box::new(annotation));
    }
}

impl fmt::Display for AnnotatedMigration {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        // TODO: Allow annotations to customize the display somehow
        fmt::Display::fmt(&self.migration, f)
    }
}

impl Migration for AnnotatedMigration {
    fn version(&self) -> &str {
        self.migration.version()
    }

    fn run(&self, conn: &SimpleConnection) -> Result<(), RunMigrationsError> {
        self.migration.run(conn)
    }

    fn revert(&self, conn: &SimpleConnection) -> Result<(), RunMigrationsError> {
        self.migration.revert(conn)
    }
    fn file_path(&self) -> Option<&Path> {
        self.migration.file_path()
    }
    fn needs_transaction(&self) -> bool {
        self.migration.needs_transaction()
    }
    fn embed(&self) -> Result<TokenStream, MigrationError> {
        // TODO: Implement embedding
        Err(MigrationError::MigrationNotEmbeddable)
    }
}

/// Iterator over migration annotations
#[derive(Debug)]
pub struct MigrationAnnotationsIter<'a>(btree_map::Values<'a, TypeId, Box<MigrationAnnotation>>);

impl<'a> Iterator for MigrationAnnotationsIter<'a> {
    type Item = &'a MigrationAnnotation;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|b| &**b)
    }
}
