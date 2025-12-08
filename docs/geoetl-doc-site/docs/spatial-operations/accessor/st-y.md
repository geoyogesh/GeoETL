---
sidebar_position: 2
title: ST_Y
description: Get Y coordinate of a Point
---

# ST_Y

Returns the Y coordinate of a Point geometry.

## Syntax

```sql
ST_Y(geometry)
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| geometry | Geometry | Point geometry |

## Returns

| Type | Description |
|------|-------------|
| Float64 | Y coordinate value |

## Examples

### Get Y Coordinate

```sql
SELECT ST_Y(ST_Point(10.5, 20.3));
-- Returns: 20.3
```

### Extract Coordinates from Points

```sql
SELECT id, ST_X(geom) as longitude, ST_Y(geom) as latitude
FROM locations;
```

### Filter by Y Range

```sql
SELECT * FROM points
WHERE ST_Y(geom) BETWEEN 37.0 AND 38.0;
```

### Calculate Distance from Equator

```sql
SELECT id, ABS(ST_Y(geom)) as distance_from_equator
FROM cities;
```

## Notes

- Only valid for Point geometries
- For geographic data, Y typically represents latitude
- Returns NULL if geometry is not a Point
- Works with both GeoArrow Point and WKB Point formats

## See Also

- [ST_X](./st-x) - Get X coordinate
- [ST_Point](../construction/st-point) - Create point from coordinates
- [ST_Centroid](../generator/st-centroid) - Get center point
