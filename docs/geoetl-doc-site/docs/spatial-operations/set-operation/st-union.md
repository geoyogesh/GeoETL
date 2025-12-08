---
sidebar_position: 1
title: ST_Union
description: Combine two geometries
---

# ST_Union

Returns the union (combination) of two geometries.

## Syntax

```sql
ST_Union(geometry_a, geometry_b)
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| geometry_a | Geometry | First geometry |
| geometry_b | Geometry | Second geometry |

## Returns

| Type | Description |
|------|-------------|
| Binary (WKB) | Union geometry as WKB with `geoarrow.wkb` metadata |

## Examples

### Union of Overlapping Polygons

```sql
SELECT ST_Union(
    ST_GeomFromText('POLYGON((0 0, 2 0, 2 2, 0 2, 0 0))'),
    ST_GeomFromText('POLYGON((1 1, 3 1, 3 3, 1 3, 1 1))')
);
-- Returns: combined L-shaped polygon
```

### Union of Points

```sql
SELECT ST_Union(ST_Point(0, 0), ST_Point(1, 1));
-- Returns: MULTIPOINT(0 0, 1 1)
```

### Merge Adjacent Parcels

```sql
SELECT ST_Union(a.geom, b.geom) as merged
FROM parcels a, parcels b
WHERE a.id = 'A' AND b.id = 'B';
```

### Dissolve Boundaries

```sql
SELECT category, ST_Union(geom) as dissolved
FROM regions
GROUP BY category;
```

## Notes

- Merges overlapping areas into a single geometry
- For non-overlapping geometries, returns a Multi* type
- Result may be a GeometryCollection if input types differ
- Use in GROUP BY for dissolving many geometries

## See Also

- [ST_Intersection](./st-intersection) - Overlapping area only
- [ST_Difference](./st-difference) - Subtract geometries
- [ST_SymDifference](./st-symdifference) - XOR operation
