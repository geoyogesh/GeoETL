---
sidebar_position: 3
title: ST_Point
description: Create point from X, Y coordinates
---

# ST_Point

Creates a Point geometry from X and Y coordinate values.

## Syntax

```sql
ST_Point(x, y)
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| x | Float64 | X coordinate (longitude in geographic CRS) |
| y | Float64 | Y coordinate (latitude in geographic CRS) |

## Returns

| Type | Description |
|------|-------------|
| FixedSizeList[2] | Point as GeoArrow Point with `geoarrow.point` metadata |

## Examples

### Create a Point

```sql
SELECT ST_Point(0, 0);
```

### Create Points from Columns

```sql
SELECT id, ST_Point(longitude, latitude) as geom FROM locations;
```

### Use in Spatial Analysis

```sql
SELECT * FROM buildings
WHERE ST_Distance(geom, ST_Point(-122.4194, 37.7749)) < 1000;
```

### Chain with Other Functions

```sql
SELECT ST_Buffer(ST_Point(x, y), 100) as buffer FROM coordinates;
```

## Notes

- Returns GeoArrow Point format (FixedSizeList[Float64, 2]) for optimal performance
- More efficient than ST_GeomFromText for creating simple points
- Coordinates are stored as-is without validation
- For 3D points with Z coordinate, use ST_GeomFromText

## See Also

- [ST_MakePoint](./st-makepoint) - Alias for ST_Point
- [ST_GeomFromText](./st-geomfromtext) - Create any geometry from WKT
- [ST_X](../accessor/st-x) - Extract X coordinate
- [ST_Y](../accessor/st-y) - Extract Y coordinate
