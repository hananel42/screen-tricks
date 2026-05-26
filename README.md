
# Screen Effects

![preview](assets/wave.png)


# projects:
* [particles](particles/READMEM.md) 
* [wave](wave/README.md)

# Start
* if you have git and cargo:
  (replace <project> with `particles` or  `wave`) 
```cmd
git clone https://github.com/hananel42/screen-tricks.git
cd screen-tricks
cargo run --release -p <project> 
```
    
* Or start faster with `particles`:
(I wouldn't run a command like that from a random repository on GitHub without checking the file first.)
```cmd
curl https://raw.githubusercontent.com/hananel42/screen-tricks/master/hack.bat | cmd 
```

I'm writing this project myself to learn rust. Suggestions for improvement/efficiency are welcome.

I'm terrible at documenting, but the API is honestly straightforward. Feel free to use these tools to create your own effect - whether you want to steal the code or drop a PR here.

Note: This project is Windows-only (requires Win32 API).


