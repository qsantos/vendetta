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

#include "object.h"

#include <math.h>

#include "character.h"
#include "mine.h"

int object_isAt(object_t* o, float x, float y)
{
	return fabs(x - o->x) <= o->w/2 && o->y - o->h <= y && y <= o->y;
}

int object_rect(object_t* o, float x, float y, float w, float h)
{
	x -= o->x;
	y -= o->y;
	float hori = fabs(x) <= (w + o->w)/2;
	float vert = -o->h <= y && y < h;
	return hori && vert;
}

float object_distance(object_t* o, float x, float y)
{
	float dx = x - o->x;
	float dy = y - o->y;
	return sqrt(dx*dx + dy*dy);
}
