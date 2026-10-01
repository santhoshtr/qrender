#!/bin/sh
# Publish the pages and their stylesheets, in the same folders as here
# (index.html at the top, v1/ and v2/ below it).
cd "$(dirname "$0")" && rsync -av -m --include='*/' --include='*.html' --include='*.css' --exclude='*' \
	./ people.codfw.wmnet:/home/santhosh/public_html/knowledge-composition/
