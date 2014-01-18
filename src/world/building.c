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

#include "building.h"

#include <string.h>

#include "../util.h"

void building_init(building_t* b, kindOf_building_t* t, character_t* owner, float x, float y)
{
	b->o.t = O_BUILDING;
	b->o.x = x;
	b->o.y = y;
	b->o.w = t->width;
	b->o.h = t->height;

	b->t = t;
	b->owner = owner;

	b->build_progress = 0;

	b->work_n = 0;
	b->work_list = 0;
	b->work_progress = 0;
}

void building_exit(building_t* b)
{
	if (b == NULL)
		return;

	free(b->work_list);
}

void building_work_enqueue(building_t* b, int c)
{
	int i = b->work_n++;
	b->work_list = CREALLOC(b->work_list, int, b->work_n);
	b->work_list[i] = c;
}

void building_work_dequeue(building_t* b, size_t n)
{
	if (n < b->work_n)
	{
		memmove(b->work_list + n, b->work_list + n + 1, sizeof(int) * (b->work_n - n -1));
		b->work_n--;
	}
}
