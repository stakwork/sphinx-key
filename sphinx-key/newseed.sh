#!/bin/bash

n=32

hexdump -vn "$n" -e ' /1 "%02x"'  /dev/urandom ; echo