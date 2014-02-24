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

#include "item.h"

#include <stdlib.h>
#include <stdio.h>

#include "../mem.h"

void kindOf_item_init(kindOf_item_t* i)
{
	i->name     = NULL;

	i->skill      = -1;
	i->category   = -1;
	effect_init(&i->effect);
	i->event      = -1;
	i->projectile = -1;
	i->range      = 0;

	transform_init(&i->cost);
	i->reqMana = 0;

	i->icon_sprite = -1;
	i->icon_index  = 0;
}

void kindOf_item_exit(kindOf_item_t* i)
{
	effect_exit(&i->effect);
	free(i->name);
}

void kindOf_item_icon(kindOf_item_t* i, assets_t* a, const char* filename, int idx)
{
	char s[1024];
	snprintf(s, 1024, "data/%s", filename);
	int id = assets_spriteId(a, s);

	i->icon_sprite = id;
	i->icon_index  = idx;
}
