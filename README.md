# Glykon's Hair

What a handsome head of hair this VST/Clap has.

## Preamble

Everything but the licensing is trés casually written because this is a trés casual project. I love all the things it touches and I endeavor to do my best to make it super rad, but all that follows is going to read pretty relaxed because these words are like the packaging it would come in, were it a box you decided to grab at a music shop because you didn't know what else to get.

## Origin story

At the beginning of 2026, I put together a bingo card with various bucket list items I wanted to accomplish. Creating a VST was one of them. I delve pretty heavily into music production and use a combination of Reaper, FL Studio, Renoise/Redux, and a roving gang whose members are not set of VSTs and plugins.

And wouldn't it be cool if I made my own VST??

The ooooonly thing that blocked me from moving forward was: what should the VST do??

"No idea," past-me said.

But then, just as I was about to fall asleep a couple nights ago, it occurred to me what could be a neat member to add to the VST/plugin gang would be a live-coding VST, where you can code various patterns, similar to redux but with everything the live-coding library would be able to do, and you could assign it to a midi note to trigger either through a midi device or through a track where the VST is treated as an instrument.

"Neat," slightly-more-future-but-still-past-me thought.

Additionally, I have a bit of a lull in one of my other side projects currently, so I figured why not work on this for a little bit.

## The name

Originally, I was thinking of making this with Strudel in mind, and I was really jazzed about naming the project "Strudeling the mid line" because it'd be real cute and punny. However, according to (AI) sources, JavaScript/TypeScript getting somehow pulled into a VST3 was not the wisest decision, what with all of the complexities of getting everything to talk to each other efficiently.

Ultimately, I got pointed in the direction of Glicol, which is a rust live coding library. Yay, I thought, but what do I call it??

I wanted a quick and easy name, and glicol made me think of Glycon, which made me think of Glycon's hair. One carefully placed capitalization later, I had figured out what to (tentatively) call it.

The idea became less tentative as I started to consider some of the implications of the name, which turns out to be a fairly good metaphor for this project.

So if you weren't already familiar, Glykon (or Glycon) is a Greco-Roman deity who is basically a snake with a really nice head of hair. A very handsome snake. However, according to our first-hand historical sources, Glycon was nothing more than a socket puppet that looked like a snake with hair (not fur, specifically hair).

As you may know, when a sock puppet moves, it's because of the hand inside of it (usually—something else might have gotten inside, but that's not the norm). And, oddly, that's a perfect analogy for what's going on here.

The sock puppet (VST wrapper) has a really nice head of hair (fancy syntax and sugar and stuff to hopefully make live-coding with the VST easier), but on the inside it's an abomination—a writhing hand uncertain of what it's doing (see me when live coding) or a deity (any number of the insanely talented live coders out there).

So yeah, that solidified it for me: "Glycon's Hair".

## The goal

In only a few words, the aim is to make a VST/Clap plugin that allows you to live-code within a DAW. There are some bonus things I want:
- Ability to switch between different kinds of Strudel/FoxDot types of syntax
- A syntax checker that lets you know when you've made a mistake with your syntax
- Easy-to-read panes that house the different commands/instruments/effects/etc.
    - Ability to click "Example" and have a fully thought-out example, in legitimate syntax, added to the text editor
- Maybe some kind of soft prediction, where if there's a limited array of things that come next, some sort of predictive placeholder reminder text??
- Ability to add samples of your own
- Lots of preset sounds modeled in glicol so samples aren't necessarily needed for basic things

## Have I reached the goal?

no.

# License

This project is licensed under the GNU General Public License v3.0 or later. See [`LICENSE`](./LICENSE) for details.

This project uses several open-source Rust libraries. In particular:

- [`glicol`](https://github.com/chaosprint/glicol), licensed under the MIT License.
- [`nih-plug`](https://github.com/robbert-vdh/nih-plug), licensed primarily under the ISC License.
- The VST3 export path used by NIH-plug relies on GPLv3-licensed VST3 bindings, so VST3 builds of this plugin are distributed under GPLv3-compatible terms.

Third-party license notices are provided in [`THIRD_PARTY_NOTICES.md`](./THIRD_PARTY_NOTICES.md), where applicable.

Unless otherwise noted, the original code in this repository is:

Copyright (C) 2026 [Your Name]

Licensed under the GNU General Public License v3.0 or later.

# Third-Party Notices

This project uses third-party open-source software. Their licenses remain their own.

## Glicol

Glicol is licensed under the MIT License.

Copyright (c) 2020-present Qichao Lan (chaosprint)

The MIT License text is reproduced below:

The MIT License (MIT)

Copyright (c) 2020-present Qichao Lan (chaopsrint)

### MIT License (for Glicol)

Permission is hereby granted, free of charge, to any person obtaining a copy of
this software and associated documentation files (the "Software"), to deal in
the Software without restriction, including without limitation the rights to
use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software is furnished to do so,
subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.[Paste the full MIT license text from Glicol here.]

## NIH-plug

NIH-plug is licensed primarily under the ISC License.

The VST3 bindings used by NIH-plug's `nih_export_vst3!()` macro are licensed under GPLv3. VST3 builds of this project are therefore distributed under GPLv3-compatible terms.

### ISC License (pertaining to NIH-plug)

ISC License

Copyright (c) 2022-2024 Robbert van der Helm

Permission to use, copy, modify, and/or distribute this software for any
purpose with or without fee is hereby granted, provided that the above
copyright notice and this permission notice appear in all copies.

THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH
REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY
AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT,
INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM
LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR
OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR
PERFORMANCE OF THIS SOFTWARE.

### GPLv3 License (pertaining to the vst3/clap parts)

GNU GENERAL PUBLIC LICENSE
Version 3, 29 June 2007

Copyright (C) 2007 Free Software Foundation, Inc. <https://fsf.org/>
Everyone is permitted to copy and distribute verbatim copies
of this license document, but changing it is not allowed.

Preamble

The GNU General Public License is a free, copyleft license for
software and other kinds of works.

The licenses for most software and other practical works are designed
to take away your freedom to share and change the works. By contrast,
the GNU General Public License is intended to guarantee your freedom to
share and change all versions of a program--to make sure it remains free
software for all its users. We, the Free Software Foundation, use the
GNU General Public License for most of our software; it applies also to
any other work released this way by its authors. You can apply it to
your programs, too.

When we speak of free software, we are referring to freedom, not
price. Our General Public Licenses are designed to make sure that you
have the freedom to distribute copies of free software (and charge for
them if you wish), that you receive source code or can get it if you
want it, that you can change the software or use pieces of it in new
free programs, and that you know you can do these things.

To protect your rights, we need to prevent others from denying you
these rights or asking you to surrender the rights. Therefore, you have
certain responsibilities if you distribute copies of the software, or if
you modify it: responsibilities to respect the freedom of others.

For example, if you distribute copies of such a program, whether
gratis or for a fee, you must pass on to the recipients the same
freedoms that you received. You must make sure that they, too, receive
or can get the source code. And you must show them these terms so they
know their rights.