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

#include "material.h"

#include <stdlib.h>
#include <stdio.h>

void kindOf_material_init(kindOf_material_t* m)
{
	m->name = NULL;
	m->skill = -1;
	m->edible = 0;
	for (int i = 0; i < N_STATUSES; i++)
		m->eatBonus[i] = 0;
}

void kindOf_material_exit(kindOf_material_t* m)
{
	free(m->name);
}

void kindOf_material_icon(kindOf_material_t* m, assets_t* a, const char* filename, int idx)
{
	char s[1024];
	snprintf(s, 1024, "data/%s", filename);
	int id = assets_spriteId(a, s);

	m->icon_sprite = id;
	m->icon_index  = idx;
}

size_t kindOf_material_info(kindOf_material_t* m, char* buffer, size_t n, universe_t* u)
{
	size_t cur = 0;

	cur += snprintf(buffer+cur, n-cur, "%s", m->name);

	if (!m->edible)
		return cur;

	cur += snprintf(buffer+cur, n-cur, "\nComestible");
	for (int i = 0; i < N_STATUSES; i++)
	{
		float b = m->eatBonus[i];
		if (b == 0)
			continue;

		char* name = u->statuses[i].name;
		cur += snprintf(buffer+cur, n-cur, "\n%+.1f %s", b, name);
	}

	return cur;
}
