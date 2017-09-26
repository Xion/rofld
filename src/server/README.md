# rofld

Lulz server

[![Build Status](https://img.shields.io/travis/Xion/rofld.svg)](https://travis-ci.org/Xion/rofld)

## What?

_rofld_ (rofl-DEE) is a mission-critical HTTP service that should be a crucial part
of any large scale, distributed infrastructure.

It makes memes.

## How?

Run it with `./cargo server run` and hit its `/caption` endpoint:

    $ ./cargo server run
    INFO: rofld v0.1.0 (rev. b707bc5)
    INFO: Starting the server to listen on 0.0.0.0:1337...

    # elsewhere
    $ curl http://127.0.0.1:1337/caption?template=zoidberg&top_text=Need%20a%20meme?&bottom_text=Why%20not%20Zoidberg?

![Need a meme? / Why not Zoidberg?](../../zoidberg.png)

Want more templates? Put them in the `data/templates` directory, duh.

## Why?

Wait, you say we'd need a _reason_ for this?

Alright, if you insist, it's for checking what's up with async Rust.
See [this post](http://xion.io/post/programming/rust-async-closer-look.html) for more details.
