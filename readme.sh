#!/bin/bash

while IFS= read -r line || [[ -n "$line" ]]; do
    while [[ "$line" =~ \{\{([^[:space:]]+)\}\} ]]; do
        key="${BASH_REMATCH[1]}"

        if [[ -f "$key" ]]; then
            content=$(cat "$key")
            line="${line//\{\{$key\}\}/$content}"
        else 
            value=$(awk -F " = " "/^$key/ {gsub(/(^\"|\"\$)/, \"\", \$2); print \$2}" Cargo.toml)
            if [[ -n "$value" ]]; then
                line="${line//\{\{$key\}\}/$value}"
            fi
        fi
    done
    echo "$line"
done < README.tpl > README.md