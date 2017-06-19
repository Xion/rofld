# rofl

Lulz on demand

[![Build Status](https://img.shields.io/travis/Xion/rofld.svg)](https://travis-ci.org/Xion/rofld)
[![Crates.io](https://img.shields.io/crates/v/rofl.svg)](http://crates.io/crates/rofl)

## What?

This here [_rofl_ crate](http://crates.io/crates/rofl) implements the tricky and complicated logic
necessary for performing the important task of putting text over pictures.

In other words, it makes memes (also known by purists as _image macros_).

## How?

How about that:

```rust
let engine = rofl::Engine::new("data/templates", "data/fonts");
let image_macro = ImageMacro {
    template: "zoidberg".into(),
    captions: vec![
        Caption::text_at(VAlign::Top, "Need a meme?"),
        Caption::text_at(VAlign::Bottom, "Why not Zoidberg?"),
    ],
    ..ImageMacro::default()
};
let output = engine.caption(image_macro)?;
let mut file = fs::OpenOptions::new().write(true).open("zoidberg.png")?;
file.write_all(&*output)?;
```

![Need a meme? / Why not Zoidberg?](../../zoidberg.png)

Neat, huh?

For an actual application using the crate, check `src/server` in [this repo](https://github.com/Xion/rofld).
