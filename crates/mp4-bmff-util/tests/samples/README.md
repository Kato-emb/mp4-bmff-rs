# Test Samples

Integration test fixtures for mp4-bmff-util.

## sample.mp4

- Codec: H.264 (Constrained Baseline)
- Resolution: 64x64
- Duration: 0.1s / 3 frames
- Frame rate: 30 fps
- Pixel format: yuv420p

```sh
ffmpeg -f lavfi -i testsrc=duration=0.1:size=64x64:rate=30 \
       -c:v libx264 -profile:v baseline -pix_fmt yuv420p \
       sample.mp4
```
