---
sidebar_position: 1
title: ST_GeomFromText
description: Parse WKT string to geometry
---

# ST_GeomFromText

Parses a Well-Known Text (WKT) string and returns a geometry.

## Syntax

```sql
ST_GeomFromText(wkt)
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| wkt | Utf8 | Well-Known Text representation of a geometry |

## Returns

| Type | Description |
|------|-------------|
| Binary (WKB) | Geometry as WKB with `geoarrow.wkb` metadata |

## Examples

### Create a Point

```sql
SELECT ST_GeomFromText('POINT(0 0)');
```

### Create a Polygon

```sql
SELECT ST_GeomFromText('POLYGON((0 0, 4 0, 4 4, 0 4, 0 0))');
```

### Parse WKT Column

```sql
SELECT id, ST_GeomFromText(wkt_column) as geom FROM raw_data;
```

### Create Complex Geometries

```sql
-- MultiPolygon
SELECT ST_GeomFromText('MULTIPOLYGON(((0 0, 1 0, 1 1, 0 1, 0 0)), ((2 2, 3 2, 3 3, 2 3, 2 2)))');

-- LineString
SELECT ST_GeomFromText('LINESTRING(0 0, 1 1, 2 0)');

-- GeometryCollection
SELECT ST_GeomFromText('GEOMETRYCOLLECTION(POINT(0 0), LINESTRING(0 0, 1 1))');
```

## Notes

- Uses GEOS library for WKT parsing
- Supports all OGC geometry types: Point, LineString, Polygon, MultiPoint, MultiLineString, MultiPolygon, GeometryCollection
- Invalid WKT will result in an error
- Case-insensitive geometry type names

## See Also

- [ST_GeomFromWKB](./st-geomfromwkb) - Create geometry from WKB
- [ST_Point](./st-point) - Create point from coordinates
