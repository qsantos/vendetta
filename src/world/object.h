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

#ifndef W_OBJECT_H
#define W_OBJECT_H

typedef enum
{
	O_CHARACTER,
	O_MINE,
	O_BUILDING,
	O_CHUNK,
	O_WORLD,
} otype_t;

typedef struct object object_t;

struct object
{
	otype_t t;

	float x;
	float y;
	float w;
	float h;
};

char  object_overlaps(object_t* o, object_t* a);
char  object_contains(object_t* o, object_t* a);
char  object_isAt    (object_t* o, float x, float y);
float object_distance(object_t* o, float x, float y);

#endif
