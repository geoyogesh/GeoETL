//! Factory system for dynamically registering and discovering format drivers.
//!
//! This module implements the Abstract Factory pattern to decouple the driver registry
//! from concrete format implementations, eliminating hard-coded dependencies.

use crate::drivers::Driver;
use crate::io::{DataReader, DataWriter};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Marker trait for format-specific options.
///
/// Format implementations should define their own option types and implement this trait.
pub trait FormatOptions: Send + Sync + 'static {
    /// Converts options to a boxed Any for dynamic dispatch.
    fn as_any(&self) -> Box<dyn std::any::Any + Send>;
}

/// Factory trait for creating format-specific readers and writers.
///
/// Each format (`CSV`, `GeoJSON`, etc.) implements this trait to provide
/// its driver metadata and I/O capabilities.
pub trait FormatFactory: Send + Sync {
    /// Returns the driver metadata for this format.
    fn driver(&self) -> Driver;

    /// Creates a reader for this format.
    ///
    /// Returns `None` if reading is not supported by this format.
    fn create_reader(&self) -> Option<Arc<dyn DataReader>>;

    /// Creates a writer for this format.
    ///
    /// Returns `None` if writing is not supported by this format.
    fn create_writer(&self) -> Option<Arc<dyn DataWriter>>;
}

/// Global registry of format factories.
///
/// This registry maintains the mapping between driver names and their factory implementations,
/// enabling dynamic format discovery and creation.
pub struct DriverRegistry {
    factories: RwLock<HashMap<String, Arc<dyn FormatFactory>>>,
}

impl DriverRegistry {
    /// Creates a new empty driver registry.
    fn new() -> Self {
        Self {
            factories: RwLock::new(HashMap::new()),
        }
    }

    /// Registers a format factory with the registry.
    ///
    /// # Arguments
    ///
    /// * `factory` - The factory implementation to register
    ///
    /// # Panics
    ///
    /// Panics if a factory with the same driver name is already registered.
    pub fn register(&self, factory: Arc<dyn FormatFactory>) {
        let driver = factory.driver();
        let name = driver.short_name.to_string();

        let mut factories = self.factories.write().unwrap();
        assert!(
            factories.insert(name.clone(), factory).is_none(),
            "Driver '{name}' is already registered"
        );
    }

    /// Finds a factory by driver name (case-insensitive).
    ///
    /// # Arguments
    ///
    /// * `name` - The driver short name to look up
    ///
    /// # Returns
    ///
    /// The factory if found, or `None` if no factory is registered for this driver.
    ///
    /// # Panics
    ///
    /// Panics if the lock is poisoned (another thread panicked while holding the lock).
    pub fn find_factory(&self, name: &str) -> Option<Arc<dyn FormatFactory>> {
        let factories = self.factories.read().unwrap();
        factories
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, factory)| Arc::clone(factory))
    }

    /// Returns all registered drivers.
    ///
    /// This includes only drivers that have been registered via format factories.
    ///
    /// # Panics
    ///
    /// Panics if the lock is poisoned (another thread panicked while holding the lock).
    pub fn get_available_drivers(&self) -> Vec<Driver> {
        let factories = self.factories.read().unwrap();
        factories.values().map(|factory| factory.driver()).collect()
    }

    /// Returns all registered driver names in sorted order.
    ///
    /// # Panics
    ///
    /// Panics if the lock is poisoned (another thread panicked while holding the lock).
    pub fn get_driver_names(&self) -> Vec<String> {
        let factories = self.factories.read().unwrap();
        let mut names: Vec<String> = factories.keys().cloned().collect();
        names.sort_unstable();
        names
    }
}

/// Returns the global driver registry instance.
///
/// This function provides access to the singleton registry where all format
/// factories are registered.
pub fn driver_registry() -> &'static DriverRegistry {
    static REGISTRY: std::sync::OnceLock<DriverRegistry> = std::sync::OnceLock::new();
    REGISTRY.get_or_init(DriverRegistry::new)
}
