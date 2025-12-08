---
sidebar_position: 8
title: ST_SimplifyPreserveTopology
description: Topology-preserving simplification
---

# ST_SimplifyPreserveTopology

Simplifies a geometry while preserving topological validity.

## Syntax

```sql
ST_SimplifyPreserveTopology(geometry, tolerance)
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

### Safe Simplification

```sql
SELECT ST_SimplifyPreserveTopology(
    ST_GeomFromText('POLYGON((0 0, 1 0, 1 0.1, 2 0, 2 2, 0 2, 0 0))'),
    0.2
);
-- Returns simplified polygon that remains valid
```

### Simplify Complex Coastlines

```sql
SELECT id, ST_SimplifyPreserveTopology(geom, 0.001) as simplified
FROM coastlines;
```

### Ensure Validity After Simplification

```sql
SELECT id,
       ST_IsValid(ST_Simplify(geom, 0.1)) as simple_valid,
       ST_IsValid(ST_SimplifyPreserveTopology(geom, 0.1)) as topo_valid
FROM parcels;
-- topo_valid is always true
```

### Progressive Simplification

```sql
SELECT
    ST_NumPoints(geom) as original,
    ST_NumPoints(ST_SimplifyPreserveTopology(geom, 0.001)) as low,
    ST_NumPoints(ST_SimplifyPreserveTopology(geom, 0.01)) as medium,
    ST_NumPoints(ST_SimplifyPreserveTopology(geom, 0.1)) as high
FROM complex_features;
```

## Notes

- Guaranteed to produce valid output from valid input
- May remove fewer points than ST_Simplify to maintain validity
- Prevents self-intersections and ring collapses
- Slightly slower than ST_Simplify due to validity checks
- Preferred for production use where validity is important

## See Also

- [ST_Simplify](./st-simplify) - Faster but may produce invalid geometries
- [ST_IsValid](../validator/st-isvalid) - Check validity
- [ST_NumPoints](../accessor/st-numpoints) - Count points
