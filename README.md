# notes

> *solution for DPG 214: Intermediate*

This is the simplest solution I could come up with, at least logic wise. I make a pile of vectors contained in a vector (because screw multi-dimensional arrays--does Rust even have those?) based on the first line of input and I make a series of "field" objects based on the rest. Each "field" object is then applied to the original pile of vectors.

I'm kind of proud of the way the program does error handling, in theory, but most of it has not been tested because I don't have a malformed input file lying around and--let's face it--I'm too lazy to make one.
