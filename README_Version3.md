# screen-tricks

**Some C++ screen tricks**

This project is a collection of tricks and techniques for working with the screen using C++. The goal is to demonstrate how to control the console display, manipulate output, and perform advanced screen operations with simple code examples.

## Main Features

- Functions for formatting and moving text on the console screen
- Clearing and deleting lines from the screen
- Controlling text and background colors
- Demos using system functions for special visual effects

## Requirements

- C++ compiler (e.g., g++, clang)
- Supported operating system (most functions tested on Windows, some may also work on Linux)

## Compilation and Running

1. **Clone the repository:**
   ```bash
   git clone https://github.com/hananel42/screen-tricks.git
   cd screen-tricks
   ```

2. **Compile and run the files:**

   - For `off.cpp` use:
     ```bash
     g++ off.cpp -o off.exe -lgdi32 -lmsimg32 -mwindows -std=c++17
     ```

   - For the other files (for example, `main.cpp`):
     ```bash
     g++ main.cpp -o main.exe -lgdi32 -luser32 -mwindows -std=c++17
     ```

   - Run the compiled executable, e.g.:
     ```bash
     ./main.exe
     ```
     or
     ```bash
     ./off.exe
     ```

## Usage

The files contain example functions. You can copy relevant parts into other projects or run the included demos.

## Contributions

Contributions, improvements, and suggestions are welcome!

## License

This project is licensed under the MIT License. See the LICENSE file for more information.