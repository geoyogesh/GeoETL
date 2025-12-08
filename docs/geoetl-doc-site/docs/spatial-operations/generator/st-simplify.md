---
sidebar_position: 7
title: ST_Simplify
description: Douglas-Peucker simplification
---

# ST_Simplify

Simplifies a geometry using the Douglas-Peucker algorithm.

## Syntax

```sql
ST_Simplify(geometry, tolerance)
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| geometry | Geometry | Input geometry |
| tolerance | Float64 | Distance tolerance for simplification |

## Returns

| Type | Description |
|------|-------------|
| Binary (WKB) | Simplified geometry as WKB with `geoarrow.wkb` metadata |

## Examples

### Simplify LineString

```sql
SELECT ST_Simplify(
    ST_GeomFromText('LINESTRING(0 0, 1 0.1, 2 0, 3 0.1, 4 0)'),
    0.2
);
-- Returns: LINESTRING(0 0, 4 0) - removes small variations
```

### Simplify Polygon

```sql
SELECT ST_Simplify(geom, 0.001) as simplified
FROM coastlines;
```

### Compare Original and Simplified

```sql
SELECT
    ST_NumPoints(geom) as original_points,
    ST_NumPoints(ST_Simplify(geom, 0.01)) as simplified_points
FROM roads;
```

### Varying Tolerance

```sql
SELECT id,
       ST_Simplify(geom, small_tolerance) as detail,
       ST_Simplify(geom, large_tolerance) as overview
FROM boundaries;
```

## Notes

- Larger tolerance = more simplification
- May produce invalid geometries (self-intersections)
- Use ST_SimplifyPreserveTopology if validity is required
- Points remain unchanged
- Tolerance is in geometry coordinate units

## See Also

- [ST_SimplifyPreserveTopology](./st-simplifypreservetopology) - Topology-safe simplification
- [ST_NumPoints](../accessor/st-numpoints) - Count points
- [ST_IsValid](../validator/st-isvalid) - Check validity
