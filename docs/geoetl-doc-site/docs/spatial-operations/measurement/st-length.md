---
sidebar_position: 3
title: ST_Length
description: Calculate length/perimeter of a geometry
---

# ST_Length

Calculates the Cartesian length of a LineString or the perimeter of a Polygon.

## Syntax

```sql
ST_Length(geometry)
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| geometry | Geometry | Input geometry |

## Returns

| Type | Description |
|------|-------------|
| Float64 | Length (for lines) or perimeter (for polygons) |

## Examples

### Length of a LineString

```sql
SELECT ST_Length(ST_GeomFromText('LINESTRING(0 0, 3 4)'));
-- Returns: 5.0
```

### Perimeter of a Square

```sql
SELECT ST_Length(ST_GeomFromText('POLYGON((0 0, 4 0, 4 4, 0 4, 0 0))'));
-- Returns: 16.0
```

### Calculate Total Road Length

```sql
SELECT road_type, SUM(ST_Length(geom)) as total_length
FROM roads
GROUP BY road_type;
```

### Find Long LineStrings

```sql
SELECT * FROM rivers
WHERE ST_Length(geom) > 1000;
```

## Notes

- Returns 0 for Points
- For Polygons, returns the perimeter (length of exterior ring plus all interior rings)
- For MultiLineStrings, returns the sum of all line lengths
- Length is in the same units as the input geometry coordinates
- For geographic coordinates, the result is in degrees (not meters)

## See Also

- [ST_Area](./st-area) - Calculate area
- [ST_NumPoints](../accessor/st-numpoints) - Count points in geometry
