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

#include "event.h"

#include <stdio.h>

void kindOf_event_init(kindOf_event_t* e)
{
	e->sprite = -1;
	e->steps = 0;
	e->duration = 1;
	e->width = 0;
	e->height = 0;
}

void kindOf_event_exit(kindOf_event_t* e)
{
	(void) e;
}

void kindOf_event_sprite(kindOf_event_t* e, graphics_t* g, const char* filename, int steps)
{
	char s[1024];
	snprintf(s, 1024, "events/%s", filename);
	int id = graphics_spriteId(g, s);

	e->sprite = id;
	e->steps = steps;

	sfIntRect rect = sfSprite_getTextureRect(g->sprites[id]);
	e->width  = rect.width / steps;
	e->height = rect.height;
}
