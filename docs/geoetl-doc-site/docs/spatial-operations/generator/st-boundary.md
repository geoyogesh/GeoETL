---
sidebar_position: 5
title: ST_Boundary
description: Compute boundary of a geometry
---

# ST_Boundary

Returns the closure of the combinatorial boundary of a geometry.

## Syntax

```sql
ST_Boundary(geometry)
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| geometry | Geometry | Input geometry |

## Returns

| Type | Description |
|------|-------------|
| Binary (WKB) | Boundary geometry as WKB with `geoarrow.wkb` metadata |

## Examples

### Polygon Boundary

```sql
SELECT ST_Boundary(ST_GeomFromText('POLYGON((0 0, 4 0, 4 4, 0 4, 0 0))'));
-- Returns: LINESTRING(0 0, 4 0, 4 4, 0 4, 0 0)
```

### LineString Boundary

```sql
SELECT ST_Boundary(ST_GeomFromText('LINESTRING(0 0, 10 10)'));
-- Returns: MULTIPOINT(0 0, 10 10) - the endpoints
```

### Closed LineString Boundary

```sql
SELECT ST_Boundary(ST_GeomFromText('LINESTRING(0 0, 4 0, 4 4, 0 0)'));
-- Returns: GEOMETRYCOLLECTION EMPTY - closed rings have empty boundary
```

### Extract Polygon Rings

```sql
SELECT id, ST_Boundary(geom) as outline
FROM parcels;
```

## Notes

- Points return empty geometry (no boundary)
- LineStrings return endpoints as MultiPoint (empty if closed)
- Polygons return the ring(s) as LineString or MultiLineString
- Boundary dimension is always one less than the input dimension
- Useful for extracting polygon outlines

## See Also

- [ST_ConvexHull](./st-convexhull) - Convex hull
- [ST_Envelope](./st-envelope) - Bounding box
- [ST_Length](../measurement/st-length) - Boundary length
