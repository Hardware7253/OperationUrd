#!/bin/bash

project_root="$PWD"

zip_gerbers() {
    cd $project_name/manufacture/
    zip -r gerbers.zip gerbers
    cd $project_root
}

projects=("microcontroller" "nixie_board" "power_supply")

for project_name in "${projects[@]}"; do
    zip_gerbers $project_name
done