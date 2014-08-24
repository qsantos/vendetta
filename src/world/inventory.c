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

#include "inventory.h"

#include <stdlib.h>
#include <string.h>

#include "../mem.h"

void inventory_init(inventory_t* inv, size_t n_materials, size_t n_items)
{
	inv->money     = 0;
	inv->materials = CALLOC(float, n_materials);
	inv->items     = CALLOC(float, n_items);

	memset(inv->materials, 0, sizeof(float)*n_materials);
	memset(inv->items,     0, sizeof(float)*n_items);
}

void inventory_exit(inventory_t* inv)
{
	free(inv->items);
	free(inv->materials);
}

float inventory_get(inventory_t* inv, char is_item, int id)
{
	return (is_item ? inv->items : inv->materials)[id];
}

void inventory_add(inventory_t* inv, char is_item, int id, float amount)
{
	if (is_item)
		inv->items[id] += amount;
	else
		inv->materials[id] += amount;
}

void inventory_mov(inventory_t* inv, char is_item, int id, float amount, inventory_t* from)
{
	inventory_add(inv,  is_item, id,  amount);
	inventory_add(from, is_item, id, -amount);
}

void inventory_pay(inventory_t* inv, float amount, inventory_t* from)
{
	inv ->money += amount;
	from->money -= amount;
}
