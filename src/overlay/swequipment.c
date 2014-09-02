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

#include "swequipment.h"

void swequipment_init(swequipment_t* w, game_t* g)
{
	subwindow_init(&w->w, g->g, "Équipement", 1024-SW_WIDTH*3, SW_HEIGHT);
}

void swequipment_exit(swequipment_t* w)
{
	subwindow_exit(&w->w);
}

int swequipment_draw(swequipment_t* w, game_t* g, char do_draw)
{
	if (do_draw)
	{
		if (!subwindow_draw(&w->w, g->g, g->a))
			return -1;
	}

	sfVector2f mouse;
	if (!do_draw)
		mouse = subwindow_mouse(&w->w, g->g);

	static sfText* text = NULL;
	if (text == NULL)
	{
		sfColor color = {255, 255, 255, 255};

		text = sfText_create();
		sfText_setFont         (text, g->a->font);
		sfText_setCharacterSize(text, 15);
		sfText_setColor        (text, color);
	}

	float x = 0;
	float y = 0;
	char caught = 0;

	// keep track of two-handed weapons (take two slots)
	char twohanded = 0;

	for (size_t i = 0; i < g->u->n_slots; i++)
	{
		kindOf_slot_t* s = &g->u->slots[i];

		// slot
		sfText_setPosition(text, (sfVector2f){x,y});
		sfText_setUTF8(text, s->name); // TODO
		if (do_draw)
			sfRenderWindow_drawText(g->g->render, text, NULL);

		int id = g->player->equipment[i];


		char* txt = NULL;
		if (id >= 0)
		{
			// icon
			kindOf_item_t* it = &g->u->items[id];
			static const int max_cols = 1024 / 32;
			int idx = it->icon_index;
			int col = idx % max_cols;
			int row = idx / max_cols;
			sfSprite* sprite = g->a->sprites[it->icon_sprite];
			sfIntRect rect = {32*col, 32*row, 32, 32};
			sfSprite_setTextureRect(sprite, rect);
			sfSprite_setPosition(sprite, (sfVector2f){x+150,y});
			if (do_draw)
				sfRenderWindow_drawSprite(g->g->render, sprite, NULL);
			else
				caught |= sfSprite_contains(sprite, mouse);

			if (it->category == 1)
				twohanded = 1;
			txt = it->name;
		}
		else if (s->category == 0 && twohanded)
		{
			twohanded = 0;
			txt = "-";
		}

		// text
		if (txt != NULL)
		{
			sfText_setPosition(text, (sfVector2f){x+182,y});
			sfText_setUTF8(text, txt); // TODO
			if (do_draw)
				sfRenderWindow_drawText(g->g->render, text, NULL);
			else
				caught |= sfText_contains(text, mouse);
		}

		y += 32;

		if (caught)
			return i;
	}

	if (do_draw)
		sfRenderWindow_setView(g->g->render, g->g->overlay_view);

	w->w.height = y;
	return -1;
}

int swequipment_cursor(swequipment_t* w, game_t* g)
{
	if (!subwindow_cursor(&w->w, g->g))
		return CURSOR_IGNORE;

	int i = swequipment_draw(w, g, 0);
	if (i < 0)
		return CURSOR_DEFAULT;

	return CURSOR_EQUIP;
}

char swequipment_catch(swequipment_t* w, game_t* g, int t)
{
	if (!subwindow_cursor(&w->w, g->g))
		return 0;

	if (t != sfMouseLeft)
		return subwindow_catch(&w->w, g->g, t);

	int i = swequipment_draw(w, g, 0);
	if (i < 0)
		return subwindow_catch(&w->w, g->g, t);

	int j = g->player->equipment[i];
	if (j >= 0)
	{
		g->player->equipment[i] = -1;
		inventory_add(&g->player->inventory, ITEM, j, 1);
	}
	return 1;
}
