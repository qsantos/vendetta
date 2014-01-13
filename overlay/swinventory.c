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

#include "swinventory.h"

#include <math.h>

#include "../util.h"

#define PANEL_N_COLS 3

void swinventory_init(swinventory_t* w, graphics_t* g)
{
	subwindow_init(&w->w, g, "Inventory", 1024-SW_WIDTH*2, 0);
}

void swinventory_exit(swinventory_t* w)
{
	subwindow_exit(&w->w);
}

size_t swinventory_materialTooltip(char* buffer, size_t n, universe_t* u, kindOf_material_t* m)
{
	size_t cur = 0;

	cur += snprintf(buffer+cur, n-cur, "%s", m->name);

	if (!m->edible)
		return cur;

	cur += snprintf(buffer+cur, n-cur, "\nedible:");
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

size_t swinventory_itemTooltip(char* buffer, size_t n, universe_t* u, kindOf_item_t* it)
{
	size_t cur = 0;

	cur += snprintf(buffer+cur, n-cur, "%s", it->name);

	for (int i = 0; i < N_SPECIAL_SKILLS; i++)
	{
		float b = it->bonus_special[i];
		if (b == 0)
			continue;

		char* name = u->sskills[i].name;
		cur += snprintf(buffer+cur, n-cur, "\n%+.1f %s", b, name);
	}

	for (size_t i = 0; i < u->n_materials; i++)
	{
		float b = it->bonus_material[i];
		if (b == 0)
			continue;

		char* name = u->materials[i].skill.name;
		cur += snprintf(buffer+cur, n-cur, "\n%+.1f %s", b, name);
	}

	for (size_t i = 0; i < u->n_iskills; i++)
	{
		float b = it->bonus_item[i];
		if (b == 0)
			continue;

		char* name = u->iskills[i].name;
		cur += snprintf(buffer+cur, n-cur, "\n%+.1f %s", b, name);
	}

	return cur;
}

int swinventory_draw(swinventory_t* w, game_t* g, char do_draw)
{
	if (do_draw)
	{
		if (!subwindow_draw(&w->w, g->g))
			return -1;
	}

	sfVector2f cursor;
	if (!do_draw)
		cursor = subwindow_mouse(&w->w, g->g);

	static sfText* text = NULL;
	if (text == NULL)
	{
		sfColor color = {255, 255, 255, 255};

		text = sfText_create();
		sfText_setFont         (text, g->g->font);
		sfText_setCharacterSize(text, 15);
		sfText_setColor        (text, color);
	}

	sfVector2f pos = {0, 0};

	for (size_t i = 0; i < g->u->n_materials; i++)
	{
		const char* name = g->u->materials[i].name;
		float amount = floor(g->player->inventory.materials[i]);

		if (amount == 0)
			continue;

		pos.y += 20;

		char buffer[1024];
		snprintf(buffer, 1024, "%s: %.0f", name, amount);

		sfText_setPosition(text, pos);
		sfText_setUTF8(text, buffer);

		if (do_draw)
		{
			sfRenderWindow_drawText(g->g->render, text, NULL);
			continue;
		}

		sfFloatRect rect = sfText_getGlobalBounds(text);
		if (sfFloatRect_contains(&rect, cursor.x, cursor.y))
			return i;
	}

	for (size_t i = 0; i < g->u->n_items; i++)
	{
		char* name = g->u->items[i].name;
		float amount = floor(g->player->inventory.items[i]);

		if (amount == 0)
			continue;

		pos.y += 20;

		char buffer[1024];
		snprintf(buffer, 1024, "%s: %.0f", name, amount);

		sfText_setPosition(text, pos);
		sfText_setUTF8(text, buffer);

		if (do_draw)
		{
			sfRenderWindow_drawText(g->g->render, text, NULL);
			continue;
		}

		sfFloatRect rect = sfText_getGlobalBounds(text);
		if (sfFloatRect_contains(&rect, cursor.x, cursor.y))
			return g->u->n_materials + i;
	}

	if (do_draw)
		sfRenderWindow_setView(g->g->render, g->g->overlay_view);

	return -1;
}

char swinventory_cursor(swinventory_t* w, game_t* g, int x, int y)
{
	if (!subwindow_cursor(&w->w, x, y))
		return 0;

	int i = swinventory_draw(w, g, 0);
	if (i < 0)
		return 1;

	char buffer[1024];
	if ((size_t) i < g->u->n_materials)
	{
		swinventory_materialTooltip(buffer, 1024, g->u, &g->u->materials[i]);
	}
	else
	{
		i -= g->u->n_materials;
		swinventory_itemTooltip(buffer, 1024, g->u, &g->u->items[i]);
	}
	graphics_drawTooltip(g->g, x, y, buffer);
	return 1;
}

char swinventory_catch(swinventory_t* w, game_t* g, int x, int y, int t)
{
	if (!subwindow_cursor(&w->w, x, y))
		return 0;

	if (t != sfMouseLeft)
		return subwindow_catch(&w->w, x, y, t);

	int i = swinventory_draw(w, g, 0);
	if (i < 0)
		return subwindow_catch(&w->w, x, y, t);

	if ((size_t) i < g->u->n_materials)
	{
		kindOf_material_t* t = &g->u->materials[i];
		if (t->edible)
		{
			g->player->inventory.materials[i]--;
			for (int i = 0; i < N_STATUSES; i++)
				g->player->statuses[i] += t->eatBonus[i];
		}

		return 1;
	}
	else
	{
		i -= g->u->n_materials;

		int cat = g->u->items[i].category;

		// TODO
		// BEGIN quickfix
		// handling two-handed items simply is tricky
		// as there is only one such category, let's hard-code it!
		// two-handed items are equiped in single-handed slot and reserve the next one
		if (cat == 0) // one-handed
		{
			char twohanded = 0; // keep track of reserved slot
			for (size_t j = 0; j < g->u->n_slots; j++)
				if (g->u->slots[j].category == 0)
				{
					int k = g->player->equipment[j];
					if (k < 0)
					{
						if (twohanded)
						{
							twohanded = 0;
						}
						else
						{
							g->player->equipment[j] = i;
							g->player->inventory.items[i]--;
							break;
						}
					}
					else if (g->u->items[k].category == 1)
					{
						twohanded = 1;
					}
				}
		}
		else if (cat == 1) // two-handed
		{
			// find two single-hand slots
			int a = -1;
			int b = -1;
			for (size_t j = 0; j < g->u->n_slots; j++)
				if (g->u->slots[j].category == 0 && g->player->equipment[j] < 0)
				{
					if (a < 0)
						a = j;
					else
					{
						b = j;
						break;
					}
				}
			if (a >= 0 && b >= 0)
			{
				g->player->equipment[a] = i;
				g->player->inventory.items[i]--;
			}
		}
		else
		// END quickfix

		for (size_t j = 0; j < g->u->n_slots; j++)
			if (g->u->slots[j].category == cat && g->player->equipment[j] < 0)
			{
				g->player->equipment[j] = i;
				g->player->inventory.items[i]--;
				break;
			}

		return 1;
	}
}
