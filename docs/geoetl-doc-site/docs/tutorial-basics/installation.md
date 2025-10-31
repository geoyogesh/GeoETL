---
sidebar_position: 1
---

# Installation Guide

This guide will help you install and set up GeoETL on your system.

## Prerequisites

- **None!** Just a computer running Linux, macOS, or Windows
- A command-line terminal

The GeoETL binary is self-contained - no additional dependencies required!

## Step 1: Download

Visit the [GeoETL Releases page](https://github.com/geoyogesh/geoetl/releases) and download the binary for your platform:

- **Linux**: `geoetl-cli-linux-x86_64.tar.gz`
- **macOS**: `geoetl-cli-macos-x86_64.tar.gz`
- **Windows**: `geoetl-cli-windows-x86_64.zip`

## Step 2: Extract

**Linux/macOS**:
```bash
tar -xzf geoetl-cli-*.tar.gz
```

**Windows**:
- Right-click the .zip file → Extract All

## Step 3: Verify

```bash
# Linux/macOS
./geoetl-cli --version

# Windows
.\geoetl-cli.exe --version
```

You should see: `geoetl 0.1.0` (or current version)

## Step 4: Add to PATH (Optional)

For convenience, add GeoETL to your system PATH so you can run it from anywhere:

**Linux/macOS**:
```bash
# Copy to a directory in your PATH
sudo mv geoetl-cli /usr/local/bin/

# Now you can run from anywhere
geoetl-cli --version
```

**Windows**:
```powershell
# Move to a permanent location
Move-Item geoetl-cli.exe C:\Program Files\GeoETL\

# Add to PATH via System Properties > Environment Variables
# Or use PowerShell (as Administrator)
$env:Path += ";C:\Program Files\GeoETL"
```

---

## Verify Setup

Let's verify everything works by listing available drivers:

```bash
# List all supported format drivers
geoetl-cli drivers

# You should see a table with 68+ drivers
```

Expected output:
```
Available Drivers (68 total):

┌─────────────────────┬──────────────────────────────┬────────────┬────────────┬────────────┐
│ Short Name          │ Long Name                    │ Info       │ Read       │ Write      │
├─────────────────────┼──────────────────────────────┼────────────┼────────────┼────────────┤
│ CSV                 │ Comma Separated Value (.csv) │ Supported  │ Supported  │ Supported  │
│ GeoJSON             │ GeoJSON                      │ Supported  │ Supported  │ Supported  │
│ ESRI Shapefile      │ ESRI Shapefile / DBF         │ Planned    │ Planned    │ Planned    │
│ ...                 │ ...                          │ ...        │ ...        │ ...        │
└─────────────────────┴──────────────────────────────┴────────────┴────────────┴────────────┘
```

## Troubleshooting

### Permission Denied

**Problem**: Cannot execute `geoetl-cli`

**Solution (Linux/macOS)**:
```bash
# Make executable
chmod +x geoetl-cli
```

### Command Not Found

**Problem**: `geoetl-cli: command not found`

**Solutions**:
- Run with full path: `./geoetl-cli`
- Or add to PATH (see "Add to PATH" section above)
- Ensure you're in the correct directory

## Updating GeoETL

To update to the latest version:

1. Download the latest release from [GitHub Releases](https://github.com/geoyogesh/geoetl/releases)
2. Extract the new binary
3. Replace your existing binary

## Uninstalling

To remove GeoETL:

```bash
# Remove binary from PATH (if you added it)
sudo rm /usr/local/bin/geoetl-cli  # Linux/macOS
```

Or simply delete the extracted `geoetl-cli` binary from wherever you saved it.

## Next Steps

Congratulations! GeoETL is now installed. 🎉

👉 **Continue to: [Your First Conversion](./first-conversion)** - Learn to convert your first geospatial file

Or explore:
- [Understanding Drivers](./understanding-drivers) - Learn about format support
- [Working with CSV](./working-with-csv) - CSV-specific operations
- [Common Operations](./common-operations) - Essential commands

## Getting Help

Need assistance?

- **Command help**: `geoetl-cli --help`
- **GitHub Issues**: [Report installation problems](https://github.com/geoyogesh/geoetl/issues)
- **GitHub Discussions**: [Ask questions](https://github.com/geoyogesh/geoetl/discussions)
