---
sidebar_position: 2
title: ST_Area
description: Calculate area of a geometry
---

# ST_Area

Calculates the Cartesian area of a geometry.

## Syntax

```sql
ST_Area(geometry)
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| geometry | Geometry | Input geometry (polygon or multipolygon) |

## Returns

| Type | Description |
|------|-------------|
| Float64 | Area of the geometry |

## Examples

### Area of a Square

```sql
SELECT ST_Area(ST_GeomFromText('POLYGON((0 0, 4 0, 4 4, 0 4, 0 0))'));
-- Returns: 16.0
```

### Area of Polygon with Hole

```sql
SELECT ST_Area(ST_GeomFromText(
    'POLYGON((0 0, 10 0, 10 10, 0 10, 0 0), (2 2, 8 2, 8 8, 2 8, 2 2))'
));
-- Returns: 64.0 (100 - 36)
```

### Calculate Total Area by Category

```sql
SELECT category, SUM(ST_Area(geom)) as total_area
FROM parcels
GROUP BY category;
```

### Filter Large Polygons

```sql
SELECT * FROM buildings
WHERE ST_Area(geom) > 1000;
```

## Notes

- Returns 0 for Points and LineStrings
- For MultiPolygons, returns the sum of all polygon areas
- Area is in squared units of the input geometry coordinates
- For geographic coordinates, the result is in square degrees (not square meters)
- Polygons with holes subtract the hole area from the outer ring

## See Also

- [ST_Length](./st-length) - Calculate length/perimeter
- [ST_Centroid](../generator/st-centroid) - Calculate center point
