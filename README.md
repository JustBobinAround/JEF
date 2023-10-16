# JEF - Jef Explore Files

**JEF** is a highly configurable file manager designed for terminal power users who demand a fast and versatile file exploration experience. With its blazing-fast fuzzy finder and extensive file-opening capabilities, JEF simplifies the task of navigating and managing your files efficiently. The program is written in Rust, using the multi-threading library Rayon for its indexing and a custom hashing algorithm to make a very fast fuzzy finder.

## Features

- **Highly Configurable**: JEF is all about customization. Tailor it to your needs by tweaking various settings, keybindings, and file opening options with the jef.toml in your config/jef.toml.

- **Fast Fuzzy Finder**: JEF features an exceptionally fast fuzzy finder to help you locate files in no time. Its custom hashing algorithm ensures rapid and accurate search results. On a solid state drive, you can expect indexing of a +/- 500,000 file root directory to take under 2 seconds.

- **Multi-threaded Indexing**: The program takes advantage of multi-threading with the Rayon library, speeding up the indexing process, and providing a smooth experience even for directories with a vast number of files.

- **TUI Application**: JEF is a TUI application, making it perfect for both keyboard enthusiasts, those who prefer a terminal-based file manager, and vim cultists alike.

- **Vim Bindings**: JEF uses Vim-style keybindings, full vim integration is still being developed. However, if you have vim set as your default terminal editor, you can open vim in the curent directoy using the **$** key. If you can exit vim (lol), you will then be returned to JEF.

## Installation

To get started with JEF, follow these steps:

1. **Prerequisites**: Make sure you have Rust and Cargo installed on your system. You can install Rust by following the instructions at [rust-lang.org](https://www.rust-lang.org/learn/get-started).

2. **Clone the Repository**: Clone the JEF repository to your local machine:

    ```shell
    git clone https://github.com/your-username/jef.git
    ```

3. **Build JEF**: Navigate to the JEF directory and build the project using Cargo:

    ```shell
    cd jef
    cargo build --release
    ```

4. **Run JEF**: You can run JEF by executing the following command:

    ```shell
    cargo run --release
    ```

Jef is still in early alpha, so no full installer has been made at this time, however you can move the target binaries into your root bin directory as a temporary solution.

## Usage

JEF is designed to be highly configurable. You can customize its behavior and appearance to match your preferences. You can find the configuration files in the JEF directory, typically named `jef.toml`. Edit these files to suit your needs.

Use the following keyboard shortcuts to navigate JEF:

- **jk**: Move through files and directories. Relative line number motions are supported.
- **Enter**: Open a file or directory.
- **Backspace**: Moves back a directory.
- **:<line_number>**: Moves to the actual line number.
- **:q**: Quit JEF.
- **/**: Activate the local finder, this will only search the current subdirectory.
- **f**: Activate the fuzzy finder, this will search current and subdirectorys.
- **Esc**: Returns to **NORMAL** mode.
- **$**: Opens the terminal's default editor in the current directory.
- **#**: Spawns a virtual shell in the current directory. Use exit or ctrl-d to return to JEF.

## Contributions

Contributions are welcome! If you encounter bugs, have ideas for new features, or want to improve the code, please submit a pull request on the JEF repository.

## License

JEF is released under the MIT License. For more information, please refer to the [LICENSE](LICENSE) file in the JEF repository.

## Acknowledgments

JEF is made possible by the hard work and dedication of its contributors. We are grateful to the Rust community, the creators of the TUI-RS crate, and everyone who supports open-source software.

---

Thank you for choosing JEF as your file manager. We hope it simplifies your file exploration and management tasks. If you have any questions or need assistance, feel free to reach out to us via the JEF repository or the contact information provided in the program's documentation.
