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

#include "../util.h"

void kindOf_item_init(kindOf_item_t* i, universe_t* u)
{
	i->name = NULL;
	i->skill = 0;
	i->category = 0;

	for (size_t k = 0; k < N_SPECIAL_SKILLS; k++)
		i->bonus_special[k] = 0;

	i->bonus_material = CALLOC(float, u->n_materials);
	for (size_t k = 0; k < u->n_materials; k++)
		i->bonus_material[k] = 0;

	i->bonus_item = CALLOC(float, u->n_iskills);
	for (size_t k = 0; k < u->n_iskills; k++)
		i->bonus_item[k] = 0;
}

void kindOf_item_exit(kindOf_item_t* i)
{
	free(i->bonus_material);
	free(i->bonus_item);
	free(i->name);
}

void kindOf_item_icon(kindOf_item_t* i, graphics_t* g, const char* filename, int idx)
{
	i->icon_sprite = graphics_spriteForImg(g, filename);
	i->icon_index  = idx;
}
