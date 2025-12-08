---
sidebar_position: 1
title: ST_IsValid
description: Test if geometry is valid according to OGC rules
---

# ST_IsValid

Tests whether a geometry is valid according to OGC Simple Features rules.

## Syntax

```sql
ST_IsValid(geometry)
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| geometry | Geometry | Input geometry to validate |

## Returns

| Type | Description |
|------|-------------|
| Boolean | True if geometry is valid, false otherwise |

## Examples

### Valid Polygon

```sql
SELECT ST_IsValid(ST_GeomFromText('POLYGON((0 0, 4 0, 4 4, 0 4, 0 0))'));
-- Returns: true
```

### Self-Intersecting Polygon (Invalid)

```sql
SELECT ST_IsValid(ST_GeomFromText('POLYGON((0 0, 2 2, 2 0, 0 2, 0 0))'));
-- Returns: false (figure-8 shape)
```

### Filter Valid Geometries

```sql
SELECT * FROM features
WHERE ST_IsValid(geom);
```

### Find Invalid Geometries

```sql
SELECT id, ST_GeometryType(geom) as type
FROM parcels
WHERE NOT ST_IsValid(geom);
```

## Notes

- Points and LineStrings are always valid
- Common invalidity causes for polygons:
  - Self-intersection (figure-8 or bowtie shapes)
  - Ring not closed
  - Holes outside shell
  - Nested holes
- Use ST_SimplifyPreserveTopology to fix some invalid geometries

## See Also

- [ST_IsSimple](./st-issimple) - Test if geometry is simple
- [ST_IsEmpty](./st-isempty) - Test if geometry is empty
- [ST_SimplifyPreserveTopology](../generator/st-simplifypreservetopology) - Fix some invalid geometries
