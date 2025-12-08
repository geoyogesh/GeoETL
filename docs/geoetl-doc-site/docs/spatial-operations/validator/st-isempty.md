---
sidebar_position: 2
title: ST_IsEmpty
description: Test if geometry is empty
---

# ST_IsEmpty

Tests whether a geometry is empty (contains no points).

## Syntax

```sql
ST_IsEmpty(geometry)
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| geometry | Geometry | Input geometry to test |

## Returns

| Type | Description |
|------|-------------|
| Boolean | True if geometry is empty, false otherwise |

## Examples

### Empty Geometry

```sql
SELECT ST_IsEmpty(ST_GeomFromText('POINT EMPTY'));
-- Returns: true
```

### Non-Empty Point

```sql
SELECT ST_IsEmpty(ST_Point(0, 0));
-- Returns: false
```

### Empty GeometryCollection

```sql
SELECT ST_IsEmpty(ST_GeomFromText('GEOMETRYCOLLECTION EMPTY'));
-- Returns: true
```

### Filter Non-Empty Geometries

```sql
SELECT * FROM features
WHERE NOT ST_IsEmpty(geom);
```

## Notes

- Empty geometries are valid but contain no coordinates
- Result of some operations may be empty (e.g., intersection of non-overlapping polygons)
- Empty geometries have zero area and length

## See Also

- [ST_IsValid](./st-isvalid) - Test geometry validity
- [ST_NumPoints](../accessor/st-numpoints) - Count points in geometry
