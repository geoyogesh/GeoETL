---
sidebar_position: 5
title: ST_GeometryType
description: Get geometry type as string
---

# ST_GeometryType

Returns the geometry type as a string.

## Syntax

```sql
ST_GeometryType(geometry)
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| geometry | Geometry | Input geometry |

## Returns

| Type | Description |
|------|-------------|
| Utf8 | Geometry type name (e.g., "Point", "Polygon") |

## Examples

### Get Type of Point

```sql
SELECT ST_GeometryType(ST_Point(0, 0));
-- Returns: "Point"
```

### Get Type of Polygon

```sql
SELECT ST_GeometryType(ST_GeomFromText('POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))'));
-- Returns: "Polygon"
```

### Group by Geometry Type

```sql
SELECT ST_GeometryType(geom) as geom_type, COUNT(*) as count
FROM features
GROUP BY ST_GeometryType(geom);
```

### Filter by Type

```sql
SELECT * FROM mixed_features
WHERE ST_GeometryType(geom) = 'Polygon';
```

## Notes

- Returns standard OGC geometry type names
- Possible values: Point, LineString, Polygon, MultiPoint, MultiLineString, MultiPolygon, GeometryCollection
- Useful for filtering or routing logic based on geometry type
- Case-sensitive return value

## See Also

- [ST_Dimension](./st-dimension) - Get topological dimension
- [ST_NumGeometries](./st-numgeometries) - Count components
