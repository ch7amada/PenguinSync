Building a sync app between android and linux using rust. The app will first implement clipboard sync between devices. 
Because clipboards are hard to read on linux, i started to implement it for fedora linux (Gnome Desktop) first, because that is what i am using.

The App is still in early stages and has for now to real functionality.

It may take a while because:
* I am still learning rust.
* Have a job.

Stack i am going to use:
* Rust (tokio, zbus ...)
* Dioxus for building the apps (still do not know if that is a good idea. I am still exploring, but i like the idea of using rust for everything)
* JS for a gnome extension (I am going to let the AI write it because it a very simple extension for getting the clipboard on gnome)
* Kotlin for some parts of the code for the android integration (also not sure how to do that)
