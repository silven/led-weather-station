# LED Weather Station

This is the repository containing the source code for my [LED Weather Station](https://silven.no/posts/58-led-display.html), for displaying various information and animations on a [64x32 RGB LED Matrix Display](https://www.adafruit.com/product/2279).

![LED Display](https://silven.no/images/led_display.png)

## Screens
There are three different screens implemented as of now. I can switch between them by long pressing the rotary knob and entering select mode, then turning the knob. Click again to exit select mode.

### Background
The main screen, downloads a couple of images and downsizes them, then you can change background image and see the sensor data scroll past.

### Waves
My Rust port of a [sweet animation](https://www.reddit.com/r/raspberry_pi/comments/hxlk9c/comment/fz8we4u) I found online.

### Maze
An animation of different randomized Mazes being explored using a depth first search.

