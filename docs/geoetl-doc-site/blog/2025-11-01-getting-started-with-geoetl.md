---
slug: getting-started-with-geoetl
title: "Getting Started with GeoETL: Your First Data Conversion"
authors: [geoyogesh]
tags: [geoetl, tutorial, cli, geospatial, conversion]
---

Welcome to your first GeoETL tutorial! In this post, we'll walk you through a simple data conversion task using the `geoetl-cli`. By the end of this tutorial, you'll be able to convert a GeoJSON file to a Parquet file.

<!-- truncate -->

## Prerequisites

Before we begin, make sure you have GeoETL installed. If you haven't already, please follow the installation instructions in our [README](https://github.com/geoyogesh/geoetl#installation).

## Step 1: Prepare your data

For this tutorial, we'll use a sample GeoJSON file. If you don't have one, you can create a simple `input.geojson` file with the following content:

```json
{
  "type": "FeatureCollection",
  "features": [
    {
      "type": "Feature",
      "properties": {
        "name": "Example Point"
      },
      "geometry": {
        "type": "Point",
        "coordinates": [-74.0060, 40.7128]
      }
    }
  ]
}
```

Save this content as `input.geojson` in your working directory.

## Step 2: Convert GeoJSON to Parquet

Now, let's convert this `input.geojson` file to a Parquet file using `geoetl-cli`. Open your terminal and run the following command:

```bash
geoetl-cli convert \
  -i input.geojson \
  -o output.parquet \
  --input-driver GeoJSON \
  --output-driver Parquet
```

Let's break down this command:
*   `geoetl-cli convert`: This is the main command to perform data conversion.
*   `-i input.geojson`: Specifies the input file, `input.geojson`.
*   `-o output.parquet`: Specifies the output file, `output.parquet`.
*   `--input-driver GeoJSON`: Tells GeoETL that the input file is in GeoJSON format.
*   `--output-driver Parquet`: Tells GeoETL to convert the data to Parquet format.

After running the command, you should see a new file named `output.parquet` in your directory.

## Step 3: Verify the conversion (Optional)

You can verify the conversion by using the `geoetl-cli info` command on your new Parquet file:

```bash
geoetl-cli info output.parquet
```

This will display information about the `output.parquet` file, confirming that the conversion was successful.

## What's Next?

You've successfully performed your first data conversion with GeoETL! This is just the beginning. GeoETL supports many other formats and operations. Explore our [User Guide](docs/USERGUIDE.md) to learn more about what you can do with GeoETL.

Stay tuned for more tutorials and updates!
