/*\
 *  Role playing, management and strategy game
 *  Copyright (C) 2013-2014 Quentin SANTOS
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with this program.  If not, see <http://www.gnu.org/licenses/>.
\*/

#ifndef STRING_H
#define STRING_H

#include <sys/types.h>
#include <stdio.h>
#include <string.h>

// those functions are in gcc's GNU extension but I like portability
char*   strdup (const char* s);
ssize_t getline(char **lineptr, size_t *n, FILE *stream);

// similar to getline(), but for tokens
ssize_t gettoken(char **tokenptr, size_t *n, FILE *stream);

#endif
