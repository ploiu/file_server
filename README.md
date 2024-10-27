# file_server

a self-hostable file backup server, written in rust because crabs are funny

please note that I don't know rust, which makes this project even funnier

## why?

Because I wanted to. Seriously though, monetized cloud storage (and cloud storage that harvests your data) is becoming
an issue. While convenient, you're willingly submitting your data to be held hostage in the event you don't pay. You
also never know _where_ your data is, _who_ is accessing it, and _what_ is being done with it. I wanted to make
something for myself to host my data without fear of big brother and others snooping on me or making me pay a fee to
access it.

## features

This project is still in development, but here is what I want from something like this:

- free (both in cost and free from 3rd party privacy intrusion)
- self-hostable
- lightweight (should be hostable on a cheap device, such as a raspberry pi)
- supports any file type
- supports flexible and powerful search based on file attributes:
    - [x] name
    - [x] user-defined tags (see [#30](https://github.com/ploiu/file_server/issues/30))
    - [ ] file type
    - [ ] date
- allows for organizing files into folders
- has a well-documented rest api

I've chosen to write this in rust with sqlite for the speed and lightness of both of those technologies. I also thought
it'd be a fun project for a language I never got to use but enjoyed the syntax of.

A well-documented api will allow anyone to build their own front end for this without fear of not knowing what they're
doing. I plan to also use this project as an opportunity to write an Android-based application for the first time.

## building
supported rustc version: 1.81.0

This project _might_ be able to run on windows, but it is primarily designed to run on a linux installation. No guarantees are made about windows. 

For building on linux, *gcc* is required to build sqlite. You will also need to add the `aarch64-unknown-linux-gnu` target using the command if you want to cross-compile for a raspberry pi.
```shell
rustup target add aarch64-unknown-linux-gnu
```

Also for cross-compiling to raspi, you will need to install aarch64 gcc. On ubuntu, you can do so with the command
```shell
sudo apt install gcc-aarch64-linux-gnu
```

## Running
all features require [rabbitmq](https://www.rabbitmq.com/) to be running on your machine. Running `docker-compose up` in the project root directory will start up a docker with rabbit, and create an admin user with username `admin` and password `admin`. To turn off rabbit-related features (such as file previews), set `RabbitMq.enabled` to `false` in `FileServer.toml`

## notes
generating file previews requires rabbitmq to be running _when this application starts_. Timing can vary depending on your device, but here's an example script that can guide you in booting up properly (works great in `/etc/rc.local`):
```sh
sudo rabbitmq-server &
# for lower end hardware, we'll need to wait before listening for a node. This is subject to hardware power
sleep 45
sudo rabbitmqctl await_online_nodes 1 && $(./sudo file_server &) &
```
