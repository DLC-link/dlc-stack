#!/bin/bash

. ./observer/build_all.sh

foreman start -f ./observer/Procfile
