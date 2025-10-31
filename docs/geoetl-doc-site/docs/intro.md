---
sidebar_position: 1
---

# Welcome to GeoETL

**GeoETL** is a modern, high-performance CLI tool for geospatial data conversion and processing, built with Rust and Apache DataFusion.

Let's get you started with **GeoETL in less than 5 minutes**.

## What is GeoETL?

GeoETL is designed to be a next-generation alternative to traditional geospatial ETL tools, offering:

- **High Performance**: 5-10x faster processing through vectorized execution
- **Memory Safety**: Built with Rust for guaranteed memory safety
- **Modern Architecture**: Leverages Apache DataFusion and Apache Arrow
- **Scalability**: From single-machine to distributed processing (coming soon)
- **68+ Format Drivers**: Support for GeoJSON, CSV, Shapefile, GeoPackage, and more

## What You'll Learn

This tutorial will teach you:

✅ **Installation** - How to install and build GeoETL
✅ **Basic Operations** - Converting data between formats
✅ **Working with Data** - Understanding drivers and formats
✅ **Advanced Features** - Performance tips and best practices

## What You'll Need

### System Requirements

- **A computer** running Linux, macOS, or Windows
- **Command-line terminal** - Terminal, PowerShell, or Command Prompt
- That's it! The pre-built binary is self-contained.

:::note Building from Source
If you want to build from source instead, you'll need Rust 1.90.0+ and Git.
See [Installation Guide](./tutorial-basics/installation) for details.
:::

### Recommended

- Basic command-line knowledge
- Familiarity with geospatial data formats (GeoJSON, CSV, etc.)
- A text editor for viewing data files

## Quick Start

Get the latest release from GitHub:

```bash
# 1. Download from GitHub Releases
# https://github.com/geoyogesh/geoetl/releases

# 2. Extract the archive
tar -xzf geoetl-cli-*.tar.gz  # Linux/macOS
# or extract the .zip on Windows

# 3. Run your first command
./geoetl-cli drivers
```

You should see a table of 68+ supported format drivers!

**→ See the [Installation Guide](./tutorial-basics/installation) for detailed step-by-step instructions.**

## Tutorial Structure

Get started with these beginner-friendly tutorials:

1. **[Installation Guide](./tutorial-basics/installation)** - Get GeoETL up and running
2. **[Your First Conversion](./tutorial-basics/first-conversion)** - Convert a GeoJSON file
3. **[Understanding Drivers](./tutorial-basics/understanding-drivers)** - Learn about format support
4. **[Working with CSV](./tutorial-basics/working-with-csv)** - CSV and WKT geometries
5. **[Common Operations](./tutorial-basics/common-operations)** - Essential commands

## Current Status

GeoETL is in **Phase 1 (Foundation)**. Here's what works today:

✅ **Working Now**:
- CSV format (read/write with WKT geometries)
- GeoJSON format (full read/write support)
- Driver registry and capability checking
- Comprehensive error messages

🚧 **Coming Soon** (Q1-Q2 2026):
- GeoPackage, Shapefile, Parquet drivers
- Spatial operations (buffer, intersection, union)
- CRS transformations
- Dataset inspection (`info` command)

See our [Roadmap](https://github.com/geoyogesh/geoetl/blob/main/docs/VISION.md) for complete details.

## Getting Help

Need assistance?

- **Documentation**: Browse these tutorials and guides
- **GitHub Issues**: [Report bugs or request features](https://github.com/geoyogesh/geoetl/issues)
- **GitHub Discussions**: [Ask questions and share ideas](https://github.com/geoyogesh/geoetl/discussions)
- **Command Help**: Run `geoetl-cli --help` or `geoetl-cli <command> --help`

## Next Steps

Ready to dive in?

👉 **[Start with the Installation Guide →](./tutorial-basics/installation)**

Or jump to:
- [Your First Conversion](./tutorial-basics/first-conversion) - Quick hands-on tutorial
- [Understanding Drivers](./tutorial-basics/understanding-drivers) - Learn format support
- [Common Operations](./tutorial-basics/common-operations) - Essential commands

---

**Let's get started!** 🚀
