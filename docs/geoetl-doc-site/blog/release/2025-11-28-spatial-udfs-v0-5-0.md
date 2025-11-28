---
slug: spatial-udfs-v0-5-0
title: "GeoETL v0.5.0: Spatial UDFs and GeoArrow Native Support"
authors: [geoyogesh]
tags: [release, spatial, geos, geoarrow, sql, v0.5.0]
date: 2025-11-28
---

**TL;DR**: GeoETL v0.5.0 adds spatial UDFs (ST_Distance, ST_Point, ST_GeomFromText) powered by GEOS for spatial operations in SQL queries during conversion.

<!--truncate-->

## What's New

v0.4.0 added SQL queries. v0.5.0 adds **spatial operations** to those queries.

**New Functions**:

| Function | Description |
|----------|-------------|
| `ST_Distance` | Minimum distance between two geometries |
| `ST_Point` / `ST_MakePoint` | Create point from X, Y coordinates |
| `ST_GeomFromText` | Parse WKT to geometry |
| `ST_GeomFromWKB` | Tag WKB binary as geometry |

**Example: Pairwise Distances**

```bash
geoetl-cli convert \
  -i cities.geojson \
  -o distances.csv \
  --input-driver GeoJSON \
  --output-driver CSV \
  --sql "SELECT a.name, b.name, ST_Distance(a.geometry, b.geometry) as dist
         FROM cities a CROSS JOIN cities b
         WHERE a.name < b.name"
```

**Example: Distance from Reference Point**

```bash
geoetl-cli convert \
  -i stores.csv \
  -o sorted.csv \
  --input-driver CSV \
  --output-driver CSV \
  --geometry-column wkt \
  --sql "SELECT name, ST_Distance(ST_GeomFromText(wkt), ST_Point(-73.98, 40.74)) as dist
         FROM stores ORDER BY dist"
```

**Example: Create Points from Columns**

```bash
geoetl-cli convert \
  -i data.csv \
  -o data.geojson \
  --input-driver CSV \
  --output-driver GeoJSON \
  --sql "SELECT name, ST_Point(lon, lat) as geometry FROM data"
```

## GeoArrow Native Support

Optimized code paths for GeoArrow types:
- `geoarrow.point` - zero-copy coordinate access
- `geoarrow.wkb` - binary geometry
- `geoarrow.geometry` - mixed types from GeoJSON

## Static GEOS Linking

Release binaries now include GEOS statically linked. No system dependencies required.

## What's Next

**v0.6.0**: `ST_Buffer`, `ST_Contains`, `ST_Intersects`, FlatGeobuf support, Shapefile read support.

See the [Roadmap](https://geoetl.com/docs/community/roadmap).

## Links

- [Installation](https://geoetl.com/docs/getting-started/installation)
- [Changelog](https://geoetl.com/docs/community/changelog#050---2025-11-28)
- [Download v0.5.0](https://github.com/geoyogesh/geoetl/releases/tag/v0.5.0)
