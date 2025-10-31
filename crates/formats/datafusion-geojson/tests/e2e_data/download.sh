#!/bin/bash

# Array of GeoArrow file URLs
GEOARROW_URLS=(
    "https://raw.githubusercontent.com/geoarrow/geoarrow-data/v0.2.0/natural-earth/files/natural-earth_cities.arrows"
    "https://raw.githubusercontent.com/geoarrow/geoarrow-data/v0.2.0/natural-earth/files/natural-earth_countries.arrows"
    "https://raw.githubusercontent.com/geoarrow/geoarrow-data/v0.2.0/natural-earth/files/natural-earth_countries-geography.arrows"
    "https://raw.githubusercontent.com/geoarrow/geoarrow-data/v0.2.0/natural-earth/files/natural-earth_countries-bounds.arrows"
)

# Loop through each URL
for url in "${GEOARROW_URLS[@]}"; do
    # 1. Extract the filename from the URL (e.g., natural-earth_cities.arrows)
    filename=$(basename "$url")

    # 2. Determine the output filename by replacing '.arrows' with '.geojson'
    output_file="${filename/.arrows/.geojson}"

    echo "Converting $filename to $output_file..."

    # 3. Execute the ogr2ogr command for conversion
    # -f GeoJSON: specifies the output format
    # "$output_file": the name of the GeoJSON file to create
    # "$url": the remote input file URL
    ogr2ogr -f GeoJSON "$output_file" "$url"

    # Check the exit status of ogr2ogr
    if [ $? -eq 0 ]; then
        echo "✅ Successfully created $output_file"
    else
        echo "❌ Error converting $filename"
    fi

    echo "---"
done

echo "Conversion process complete."
