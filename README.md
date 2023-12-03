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
