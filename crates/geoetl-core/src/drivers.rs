//! Static registry describing `GeoETL` driver capabilities.
//!
//! This module enumerates every known driver, including planned support
//! statuses, and exposes helper functions for querying the registry.

/// Represents the support status for a specific driver operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportStatus {
    /// The feature is fully supported and implemented.
    Supported,
    /// The feature is not supported by the driver.
    NotSupported,
    /// The feature is planned for future implementation.
    Planned,
}

impl SupportStatus {
    /// Returns true if the status is Supported
    #[must_use]
    pub fn is_supported(&self) -> bool {
        matches!(self, SupportStatus::Supported)
    }

    /// Returns true if the status is not `NotSupported` (i.e., Supported or Planned)
    #[must_use]
    pub fn is_available(&self) -> bool {
        !matches!(self, SupportStatus::NotSupported)
    }

    /// Display string for this status
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            SupportStatus::Supported => "Supported",
            SupportStatus::NotSupported => "Not Supported",
            SupportStatus::Planned => "Planned",
        }
    }
}

/// Capabilities that a driver can support
#[derive(Debug, Clone, Copy)]
pub struct DriverCapabilities {
    /// Can read information about the dataset
    pub info: SupportStatus,
    /// Can read data from this format
    pub read: SupportStatus,
    /// Can write data to this format
    pub write: SupportStatus,
}

impl DriverCapabilities {
    /// Returns true if at least one operation is supported or planned
    #[must_use]
    pub fn has_any_support(&self) -> bool {
        self.info.is_available() || self.read.is_available() || self.write.is_available()
    }

    /// Returns true if at least one operation is fully supported
    #[must_use]
    pub fn has_supported_operation(&self) -> bool {
        self.info.is_supported() || self.read.is_supported() || self.write.is_supported()
    }
}

/// Vector format driver definition
#[derive(Debug, Clone)]
pub struct Driver {
    /// Short name used in CLI (e.g., `GeoJSON`)
    pub short_name: &'static str,
    /// Long descriptive name (e.g., `GeoJSON`)
    pub long_name: &'static str,
    /// Supported operations
    pub capabilities: DriverCapabilities,
}

impl Driver {
    /// Create a new driver definition
    #[must_use]
    pub const fn new(
        short_name: &'static str,
        long_name: &'static str,
        info: SupportStatus,
        read: SupportStatus,
        write: SupportStatus,
    ) -> Self {
        Self {
            short_name,
            long_name,
            capabilities: DriverCapabilities { info, read, write },
        }
    }
}

/// Registry of all supported vector drivers
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn get_drivers() -> Vec<Driver> {
    use SupportStatus::{NotSupported, Planned, Supported};

    vec![
        // Core formats - Phase 2 implementation
        Driver::new("GeoJSON", "GeoJSON", Supported, Supported, Supported),
        Driver::new(
            "GeoJSONSeq",
            "GeoJSONSeq: sequence of GeoJSON features",
            Planned,
            Planned,
            Planned,
        ),
        Driver::new(
            "ESRI Shapefile",
            "ESRI Shapefile / DBF",
            Planned,
            Planned,
            Planned,
        ),
        Driver::new("GPKG", "GeoPackage vector", Planned, Planned, Planned),
        Driver::new("FlatGeobuf", "FlatGeobuf", Planned, Planned, Planned),
        Driver::new("Parquet", "(Geo)Parquet", Supported, Supported, Supported),
        Driver::new(
            "Arrow",
            "(Geo)Arrow IPC File Format / Stream",
            Planned,
            Planned,
            Planned,
        ),
        // Common interchange formats
        Driver::new(
            "GML",
            "Geography Markup Language",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "KML",
            "Keyhole Markup Language",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "LIBKML",
            "LIBKML Driver (.kml .kmz)",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "GPX",
            "GPS Exchange Format",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "CSV",
            "Comma Separated Value (.csv)",
            Planned,
            Planned,
            Planned,
        ),
        Driver::new(
            "GeoRSS",
            "GeoRSS: Geographically Encoded Objects for RSS",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "GMT",
            "GMT ASCII Vectors (.gmt)",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "JSONFG",
            "OGC Features and Geometries JSON",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        // CAD formats
        Driver::new(
            "DXF",
            "AutoCAD DXF",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "DWG",
            "AutoCAD DWG",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "CAD",
            "AutoCAD DWG",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "DGN",
            "Microstation DGN",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "DGNv8",
            "Microstation DGN v8",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        // ESRI formats
        Driver::new(
            "FileGDB",
            "ESRI File Geodatabase (FileGDB)",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "OpenFileGDB",
            "ESRI File Geodatabase vector (OpenFileGDB)",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "PGeo",
            "ESRI Personal Geodatabase",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "ESRIJSON",
            "ESRIJSON / FeatureService driver",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        // Database formats
        Driver::new(
            "PostgreSQL",
            "PostgreSQL/PostGIS",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "PGDump",
            "PostgreSQL SQL Dump",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new("MySQL", "MySQL", NotSupported, NotSupported, NotSupported),
        Driver::new(
            "SQLite",
            "SQLite / Spatialite",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "ODBC",
            "ODBC RDBMS",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "MSSQLSpatial",
            "Microsoft SQL Server Spatial Database",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "OCI",
            "Oracle Spatial",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new("HANA", "SAP HANA", NotSupported, NotSupported, NotSupported),
        Driver::new(
            "MongoDBv3",
            "MongoDBv3",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        // Web services
        Driver::new(
            "WFS",
            "OGC WFS (Web Feature Service)",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "OAPIF",
            "OGC API - Features",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "CSW",
            "OGC CSW (Catalog Service for the Web)",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new("CARTO", "Carto", NotSupported, NotSupported, NotSupported),
        Driver::new(
            "AmigoCloud",
            "AmigoCloud",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "Elasticsearch",
            "Elasticsearch: Geographically Encoded Objects",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "EEDA",
            "Google Earth Engine Data API",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "NGW",
            "NextGIS Web",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        // Other formats
        Driver::new(
            "MapInfo File",
            "MapInfo TAB and MIF/MID",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "OSM",
            "OpenStreetMap XML and PBF",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "MVT",
            "MVT: Mapbox Vector Tiles",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "PDF",
            "Geospatial PDF",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "ODS",
            "Open Document Spreadsheet",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "XLSX",
            "Microsoft Office Excel",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "netCDF",
            "NetCDF: Network Common Data Form - Vector",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new("MapML", "MapML", NotSupported, NotSupported, NotSupported),
        Driver::new(
            "MEM",
            "In Memory datasets",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "GPSBabel",
            "GPSBabel",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "GTFS",
            "General Transit Feed Specification",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "GRASS",
            "GRASS Vector Format",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "JML",
            "OpenJUMP JML format",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "VDV",
            "VDV-451/VDV-452/INTREST Data Format",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "IDRISI",
            "Idrisi Vector (.VCT)",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "AVCE00",
            "Arc/Info E00 (ASCII) Coverage",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "AVCBIN",
            "Arc/Info Binary Coverage",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new("NAS", "ALKIS", NotSupported, NotSupported, NotSupported),
        Driver::new(
            "LVBAG",
            "Dutch Kadaster LV BAG 2.0 Extract",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new("EDIGEO", "EDIGEO", NotSupported, NotSupported, NotSupported),
        Driver::new(
            "MiraMonVector",
            "MiraMon Vector",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "PDS",
            "Planetary Data Systems TABLE",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "GMLAS",
            "GML driven by application schemas",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "INTERLIS",
            "INTERLIS 1 and 2 drivers",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "ADBC",
            "Arrow Database Connectivity",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "AIVector",
            "Artificial intelligence powered vector driver",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
        Driver::new(
            "GDALG",
            "GDAL Streamed Algorithm",
            NotSupported,
            NotSupported,
            NotSupported,
        ),
    ]
}

/// Get all drivers that have at least one supported operation
#[must_use]
pub fn get_available_drivers() -> Vec<Driver> {
    get_drivers()
        .into_iter()
        .filter(|d| d.capabilities.has_supported_operation())
        .collect()
}

/// Find a driver by its short name (case-insensitive)
#[must_use]
pub fn find_driver(name: &str) -> Option<Driver> {
    get_drivers()
        .into_iter()
        .find(|d| d.short_name.eq_ignore_ascii_case(name))
}

/// List all drivers that support a specific capability
#[must_use]
pub fn list_drivers_with_capability(read: bool, write: bool, info: bool) -> Vec<Driver> {
    get_drivers()
        .into_iter()
        .filter(|d| {
            let read_ok = !read || d.capabilities.read.is_supported();
            let write_ok = !write || d.capabilities.write.is_supported();
            let info_ok = !info || d.capabilities.info.is_supported();
            read_ok && write_ok && info_ok
        })
        .collect()
}

/// Get all driver short names as a sorted list
#[must_use]
pub fn get_driver_names() -> Vec<&'static str> {
    let mut names: Vec<_> = get_drivers().iter().map(|d| d.short_name).collect();
    names.sort_unstable();
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_driver() {
        let driver = find_driver("GeoJSON");
        assert!(driver.is_some());
        assert_eq!(driver.unwrap().short_name, "GeoJSON");
    }

    #[test]
    fn test_find_driver_case_insensitive() {
        let driver = find_driver("geojson");
        assert!(driver.is_some());
        assert_eq!(driver.unwrap().short_name, "GeoJSON");
    }

    #[test]
    fn test_list_read_write_drivers() {
        let drivers = list_drivers_with_capability(true, true, false);
        // GeoJSON and Parquet are supported
        assert_eq!(drivers.len(), 2);
        assert!(drivers.iter().any(|d| d.short_name == "GeoJSON"));
        assert!(drivers.iter().any(|d| d.short_name == "Parquet"));
    }

    #[test]
    fn test_available_drivers() {
        let drivers = get_available_drivers();
        // Should have drivers with at least one Supported operation
        assert_eq!(drivers.len(), 2);
        assert!(drivers.iter().any(|d| d.short_name == "GeoJSON"));
        assert!(drivers.iter().any(|d| d.short_name == "Parquet"));
    }

    #[test]
    fn test_support_status() {
        assert!(SupportStatus::Supported.is_supported());
        assert!(!SupportStatus::NotSupported.is_supported());
        assert!(!SupportStatus::Planned.is_supported());

        assert!(SupportStatus::Supported.is_available());
        assert!(!SupportStatus::NotSupported.is_available());
        assert!(SupportStatus::Planned.is_available());
    }
}
