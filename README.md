# Soccer

Autonomous soccer robots for RoboCupJunior Soccer Open. We created these robots and participated in the RoboCup Singapore 2024 competition as one of the teams from Raffles Institution. This repository contains the main software powering our robots. We achieved first place in Singapore, and we will likely be competing internationally in the Netherlands!

Here is a photo from the competition, taken by the RoboCup Media Team.

![image](https://github.com/chamburr/soccer/assets/42373024/d29cedb3-72c8-454b-a450-4f0c9766ec31)

## Hardware

The main hardware we have consists of 4 ToF LiDARs, an IMU with a magnetometer, a low-resolution camera, 4 motors, and several ambient light sensors. In an attempt to keep our robots simple and reliable, we unfortunately do not have anything special like kickers and dribblers. Hardware files are available at [this repository](https://github.com/PorridgePi/soccer-pcbs).

### Microcontrollers

We also have 4 microcontrollers in each robot.

- **STM32F411CE**: This is only used for reading analogue data from the ambient light sensors and runs very simple Arduino code. Data is sent to the main microcontroller through 5 digital pins.
- **STM32H743VI**: This is used by the OpenMV H7 camera for image processing and runs MicroPython with OpenMV-specific libraries. Data is sent to the main microcontroller through UART.
- **RP2040 (RP2040-Zero)**: This is used for reading data from the LiDARs over I2C, as well as for running IMU sensor fusion algorithms. Data is sent to the main microcontroller through UART.
- **RP2040 (Pico W)**: This is our main microcontroller, it runs everything else from movement control to localisation, to strategy code, and an online control panel. :)

## Software

We will only document the software running on the RP2040s, as the rest are pretty boring. As you may have already noticed, the main programming language in this repository is Rust :crab:. We chose to adopt Rust with the Embassy crate for this project, as opposed to conventional C++ with Arduino, and here are some of the key reasons.

- **Asynchronous programming**. This is the single most attractive feature, especially when running bare metal. It allows us to run many tasks concurrently without having to worry about performance, and it also makes it much easier for project structuring.
- **Powerful type system**. The type system in Rust and Embassy ensures that most bugs are caught at compile time. This saves us significant time from debugging, as most of the time, the code works as long as it compiles successfully.
- **Memory and thread safety**. With first-class support for memory and thread safety features such as mutexes and pub/sub channels, it is very easy to add more complex features to our code, while fully utilising the multi-core hardware environment.
- **Embedded Rust ecosystem**. While embedded Rust remains experimental due to its low adoption rates, a robust ecosystem is currently under very active development. For instance, we were able to build an online robot control panel thanks to support for the wireless functionality on the Pico W, and the availability of HTTP server libraries for embedded environments.
- **Rust is fun**. Last but not least, we didn't just want to write code that works, but we also wanted to enjoy the process of it. As our all-time favourite language, choosing Rust is almost a no-brainer.

### Project structure

This only applies to the software running on the Pico W, which can be found inside the `soccer-main` folder. We structured our project while following 3 layers of abstraction -- hardware, modules and strategy. This structure allows us to write clearer and more concise code, which is very helpful when the codebase becomes bigger.

- **Hardware**. This is the only layer that interfaces with the hardware, handling the communication protocols. It exposes mutexes and channels for retrieving data and controlling the hardware at a very low level.
- **Modules**. This layer combines data received from various hardware into more useful information and exposes them using mutexes and channels. It also exposes higher-level APIs for controlling the hardware.
- **Strategy**. This is the layer handling all of the main logic, such as attack and defence. It only interacts with the module layer, so we don't have to think about the underlying hardware when writing strategy code.

### Asynchronous programming

We mentioned that asynchronous programming is the most attractive feature in Rust and Embassy because it lets us save precious I/O wait time to execute more real-time code, and allows us to better structure our project. Running our code wouldn't be otherwise possible in such a constrained environment. Here is some information on how we use asynchronous programming.

- We run many, many threads on each robot. Individual threads are used for polling each of the hardware components. Additionally, we have threads for each module, as well as threads for networking and strategy.
- We use mutexes whenever we want to communicate information to multiple threads. For real-time data syncing, we use pub/sub channels with empty messages to alert threads that new data is available.
- We use signals, which are pub/sub channels that only keep the latest value, for controlling hardware and modules. For instance, if we want to control a motor's speed, we would publish a value into that motor's signal.
- Finally, we have a thread that runs code to decide which strategy to use whenever new data is available. As only one strategy is used at a time, we decided against having a thread for each strategy. Instead, we wrote functions that can be run momentarily, and have the main thread call the functions while passing in data and persistent state.

## Caveats

Here are several software issues we encountered while writing code for our robots.

- The OpenMV camera is extremely painful to use, with significant bugs in its software. Many features written in their documentation are completely broken, and they will only admit them when someone raises questions in their forums.
- The Pico W has very slow wireless networking. In noisy areas, many packets are lost even when the router is nearby. Also, running networking tasks on the Pico W severely limits processing power for other tasks.
- Embassy and the embedded Rust ecosystem are still in early development, and we experienced many bugs. Nevertheless, it was fun to use cutting-edge technology, and we believe that the embedded Rust ecosystem has a very bright future.

## License

This project is licensed under the [MIT License](LICENSE).
