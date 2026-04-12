# Image Thumbnail Service

<!-- Mid-size fixture: one paragraph of intent + a sketched flow. -->

## Intent

Take an uploaded image, produce three thumbnails at fixed widths,
and store them addressably by content hash so identical uploads
share storage.

## Sketch

- accept an image upload from the user
- validate the file is a supported format
- decode the image into a pixel buffer
- resize to small medium and large widths
- encode each thumbnail as avif
- hash each avif blob with sha-256
- store each blob keyed by its hash
- return the three hashes to the caller
