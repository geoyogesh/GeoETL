---
sidebar_position: 1
title: ST_Buffer
description: Create buffer polygon around geometry
---

# ST_Buffer

Creates a buffer polygon at a specified distance around a geometry.

## Syntax

```sql
ST_Buffer(geometry, distance)
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| geometry | Geometry | Input geometry |
| distance | Float64 | Buffer distance in geometry units |

## Returns

| Type | Description |
|------|-------------|
| Binary (WKB) | Buffer polygon as WKB with `geoarrow.wkb` metadata |

## Examples

### Buffer Around Point

```sql
SELECT ST_Buffer(ST_Point(0, 0), 10.0);
-- Returns a circle polygon with radius 10
```

### Buffer Around LineString

```sql
SELECT ST_Buffer(ST_GeomFromText('LINESTRING(0 0, 10 0)'), 5.0);
-- Returns a rounded rectangle
```

### Buffer with Column Distance

```sql
SELECT id, ST_Buffer(geom, buffer_radius) as buffer_zone
FROM features;
```

### Negative Buffer (Polygon Shrinking)

```sql
SELECT ST_Buffer(
    ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))'),
    -2.0
);
-- Returns a smaller polygon inset by 2 units
```

## Notes

- Positive distance creates an outward buffer
- Negative distance creates an inward buffer (for polygons only)
- Buffer distance is in the same units as the geometry coordinates
- For geographic coordinates, distance is in degrees
- Uses GEOS library for computation with default segment count

## See Also

- [ST_Centroid](./st-centroid) - Get center point
- [ST_Envelope](./st-envelope) - Get bounding box
- [ST_Distance](../measurement/st-distance) - Measure distance
