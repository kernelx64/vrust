# vrust 🦀☀️

**vrust** is a personal monitoring tool developed in Rust to interface with Victron Energy MPPT charge controllers using the VE.Direct protocol.

## 🚀 Overview
This project is part of my 2026 learning journey into Rust. It runs on my Linux Tumbleweed environment (nicknamed "Achiever") and automates the collection of solar harvest data, which was previously logged manually in the `ddata_26` project.

## 🛠 Features (Planned)
- **Serial Communication:** Reading VE.Direct HEX/Text protocols via USB-Serial.
- **Data Logging:** Storing Yield (Wh) and Pmax (W) values.
- **Automation:** Future integration with Google Sheets for automated solar reporting.
- **Lightweight:** Minimal footprint, designed to run 24/7.

## 💻 Tech Stack
- **Language:** Rust 1.x
- **Platform:** Linux (OpenSUSE Tumbleweed)
- **Hardware:** Victron MPPT + VE.Direct to USB Cable.

---
*Developed by Adelino Saldanha as a personal achievement project.*
