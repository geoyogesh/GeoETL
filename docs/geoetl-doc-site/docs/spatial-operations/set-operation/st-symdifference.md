---
sidebar_position: 4
title: ST_SymDifference
description: Symmetric difference (XOR) of geometries
---

# ST_SymDifference

Returns the symmetric difference of two geometries - the portions that are in either geometry but not in both.

## Syntax

```sql
ST_SymDifference(geometry_a, geometry_b)
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| geometry_a | Geometry | First geometry |
| geometry_b | Geometry | Second geometry |

## Returns

| Type | Description |
|------|-------------|
| Binary (WKB) | Symmetric difference as WKB with `geoarrow.wkb` metadata |

## Examples

### XOR of Overlapping Polygons

```sql
SELECT ST_SymDifference(
    ST_GeomFromText('POLYGON((0 0, 2 0, 2 2, 0 2, 0 0))'),
    ST_GeomFromText('POLYGON((1 1, 3 1, 3 3, 1 3, 1 1))')
);
-- Returns: two separate polygons (non-overlapping parts)
```

### Find Non-Overlapping Areas

```sql
SELECT ST_SymDifference(old_boundary.geom, new_boundary.geom) as changed_area
FROM boundaries old_boundary, boundaries new_boundary
WHERE old_boundary.year = 2020 AND new_boundary.year = 2024;
```

### Compare Versions

```sql
SELECT
    ST_Area(ST_SymDifference(v1.geom, v2.geom)) as difference_area,
    ST_Area(ST_Intersection(v1.geom, v2.geom)) as unchanged_area
FROM parcels_v1 v1, parcels_v2 v2
WHERE v1.id = v2.id;
```

### Symmetric Result

```sql
SELECT
    ST_Equals(
        ST_SymDifference(a.geom, b.geom),
        ST_SymDifference(b.geom, a.geom)
    ) as is_symmetric
FROM regions a, regions b;
-- Always returns true
```

## Notes

- Equivalent to (A - B) UNION (B - A)
- Operation is commutative: ST_SymDifference(A, B) = ST_SymDifference(B, A)
- Returns empty geometry if A equals B
- Useful for change detection between versions

## See Also

- [ST_Difference](./st-difference) - One-way subtraction
- [ST_Intersection](./st-intersection) - Shared area
- [ST_Union](./st-union) - Combined area
