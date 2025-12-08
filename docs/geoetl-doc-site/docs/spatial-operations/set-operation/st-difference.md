---
sidebar_position: 3
title: ST_Difference
description: Difference of geometries (A - B)
---

# ST_Difference

Returns the portion of geometry A that does not intersect with geometry B.

## Syntax

```sql
ST_Difference(geometry_a, geometry_b)
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| geometry_a | Geometry | Geometry to subtract from |
| geometry_b | Geometry | Geometry to subtract |

## Returns

| Type | Description |
|------|-------------|
| Binary (WKB) | Difference geometry as WKB with `geoarrow.wkb` metadata |

## Examples

### Cut Hole in Polygon

```sql
SELECT ST_Difference(
    ST_GeomFromText('POLYGON((0 0, 4 0, 4 4, 0 4, 0 0))'),
    ST_GeomFromText('POLYGON((1 1, 3 1, 3 3, 1 3, 1 1))')
);
-- Returns: polygon with rectangular hole
```

### Remove Overlap

```sql
SELECT ST_Difference(
    ST_GeomFromText('POLYGON((0 0, 2 0, 2 2, 0 2, 0 0))'),
    ST_GeomFromText('POLYGON((1 0, 3 0, 3 2, 1 2, 1 0))')
);
-- Returns: POLYGON((0 0, 1 0, 1 2, 0 2, 0 0))
```

### Exclude Protected Areas

```sql
SELECT id, ST_Difference(parcel.geom, protected.geom) as developable
FROM parcels parcel, protected_areas protected
WHERE ST_Intersects(parcel.geom, protected.geom);
```

### Order Matters

```sql
SELECT
    ST_Area(ST_Difference(a.geom, b.geom)) as a_minus_b,
    ST_Area(ST_Difference(b.geom, a.geom)) as b_minus_a
FROM regions a, regions b
WHERE a.name = 'Region A' AND b.name = 'Region B';
```

## Notes

- A - B is not the same as B - A (order matters)
- Returns A unchanged if geometries don't intersect
- Returns empty geometry if B completely contains A
- Useful for creating exclusion zones and cutting holes

## See Also

- [ST_SymDifference](./st-symdifference) - XOR (A-B union B-A)
- [ST_Intersection](./st-intersection) - Shared area
- [ST_Union](./st-union) - Combined area
