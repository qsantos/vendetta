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

void inventory_init(inventory_t* i, size_t n_materials, size_t n_items)
{
	i->materials = CALLOC(float, n_materials);
	i->items     = CALLOC(float, n_items);

	memset(i->materials, 0, sizeof(float)*n_materials);
	memset(i->items,     0, sizeof(float)*n_items);
}

void inventory_exit(inventory_t* i)
{
	free(i->items);
	free(i->materials);
}
