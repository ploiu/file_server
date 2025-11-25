# file_server

a self-hostable file backup server, written in rust because crabs are funny

please note that I don't know rust, which makes this project even funnier

## why?

Because I wanted to. Seriously though, monetized cloud storage (and cloud
storage that harvests your data or uses it to train AIs) is becoming an issue.
While convenient, you're willingly submitting your data to be held hostage in
the event you don't pay. You also never know _where_ your data is, _who_ is
accessing it, and _what_ is being done with it. I wanted to make something for
myself to host my data without fear of big brother and others snooping on me or
making me pay a fee to access it.

## features

- free (as in beer, duh)
- self hostable on lower end devices (I run mine on a raspberry pi 3B)
- flexible search and tagging functionality
- well documented api via openapi docs
- flexible (but simple!) config
- portable - you can zip up the entire directory the server is running in, unzip
  it on a new device, and it will run fine (though you may need to change your
  certificates). This makes manual backups easy

I've chosen to write this in rust with sqlite for the speed and lightness of
both of those technologies. I also thought it'd be a fun project for a language
I never got to use but enjoyed the syntax of.

## building

supported rustc version: 1.88.0

This project _might_ be able to run on windows, but it is primarily designed to
run on a linux installation. No guarantees are made about windows.

For building on linux, _gcc_ is required to build sqlite. You will also need to
add the `aarch64-unknown-linux-gnu` target using the command if you want to
cross-compile for a raspberry pi.

```shell
rustup target add aarch64-unknown-linux-gnu
```

Also for cross-compiling to raspi, you will need to install aarch64 gcc. On
ubuntu, you can do so with the command

```shell
sudo apt install gcc-aarch64-linux-gnu
```

## Running

```shell
cargo run
```

preview generation for file uploads requires [ffmpeg](https://ffmpeg.org/) and
[rabbitmq](https://www.rabbitmq.com/) to be running on your machine. Running
`docker compose up` in the project root directory will start up a docker with
rabbit, and create an admin user with username `admin` and password `admin`. To
turn off this feature, set `RabbitMq.enabled` to `false` in `FileServer.toml`

## Testing

- some tests require ffmpeg to run (specifically all tests for generating file
  previews)
- the same tests require these files located in `./test_assets`: `test.png`,
  `test.gif`, `test.mp4` They are ignored from git to reduce repository size and
  because I don't want to be liable if it turns out test data I used is
  copyrighted

## notes

generating file previews requires rabbitmq to be running _when this application
starts_. Timing can vary depending on your device, but here's an example script
that can guide you in booting up properly (works great in `/etc/rc.local`):

```shell
sudo rabbitmq-server &
# for lower end hardware, we'll need to wait before listening for a node. you may need to adjust the timing
sleep 45
sudo rabbitmqctl await_online_nodes 1 && $(./sudo file_server &) &
```

you can also use `systemd` to ensure this launches after rabbit
