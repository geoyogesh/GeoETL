# GeoETL Documentation

Comprehensive documentation for the GeoETL geospatial data processing toolkit.

## Quick Links

### For Users

- **[CSV User Guide](formats/csv-user-guide.md)** - Reading CSV files with WKT geometries
- **[GeoJSON User Guide](formats/geojson-user-guide.md)** - Working with GeoJSON data
- **[Integration Guide](DATAFUSION_GEOSPATIAL_FORMAT_INTEGRATION_GUIDE.md)** - Implementing custom format integrations

### For Developers

- **[CSV Development Guide](formats/csv-development.md)** - CSV format implementation details
- **[GeoJSON Development Guide](formats/geojson-development.md)** - GeoJSON format implementation details
- **[Documentation Review](DOCUMENTATION_REVIEW.md)** - Documentation accuracy and recommendations

## Documentation Structure

```
docs/
├── README.md (this file)
├── DATAFUSION_GEOSPATIAL_FORMAT_INTEGRATION_GUIDE.md
├── DOCUMENTATION_REVIEW.md
└── formats/
    ├── csv-user-guide.md
    ├── csv-development.md
    ├── geojson-user-guide.md
    └── geojson-development.md
```

## Getting Started

### New Users

1. Start with the format-specific user guides:
   - [CSV User Guide](formats/csv-user-guide.md) if you have CSV data with WKT geometries
   - [GeoJSON User Guide](formats/geojson-user-guide.md) if you have GeoJSON data

2. Each guide includes:
   - Quick start examples
   - Configuration options
   - Cloud storage usage
   - Performance tips
   - Troubleshooting

### Developers

1. Read the [Integration Guide](DATAFUSION_GEOSPATIAL_FORMAT_INTEGRATION_GUIDE.md) for overview
2. Study the format-specific development guides for implementation details
3. Review the [Documentation Review](DOCUMENTATION_REVIEW.md) for latest updates and recommendations

## Supported Formats

| Format | Status | User Guide | Dev Guide |
|--------|--------|------------|-----------|
| CSV with WKT | ✅ Stable | [Link](formats/csv-user-guide.md) | [Link](formats/csv-development.md) |
| GeoJSON | ✅ Stable | [Link](formats/geojson-user-guide.md) | [Link](formats/geojson-development.md) |
| GeoParquet | 🚧 In Progress | TBD | TBD |
| FlatGeobuf | 🚧 In Progress | TBD | TBD |

## Contributing

See the development guides for contribution guidelines:
- [CSV Development Guide - Contributing](formats/csv-development.md#contributing)
- [GeoJSON Development Guide - Contributing](formats/geojson-development.md#contributing)

## Version Information

**Current Documentation Version**: Based on geoetl v0.1.0

| Component | Version |
|-----------|---------|
| DataFusion | 50.1.0 |
| Arrow | 56 |
| geoarrow | 0.6.1 |

See [Documentation Review](DOCUMENTATION_REVIEW.md) for version compatibility matrix.

## External Resources

- [GeoArrow Specification](https://geoarrow.org/)
- [GeoArrow Rust Documentation](https://geoarrow.org/geoarrow-rs/rust/)
- [DataFusion User Guide](https://datafusion.apache.org/user-guide/)
- [Apache Arrow](https://arrow.apache.org/)
- [Natural Earth Test Data](https://geoarrow.org/data.html)
