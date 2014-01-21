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

#include "menu.h"

#include <stdlib.h>

#include "graphics.h"
#include "game.h"

static char draw_button(float x, float y, const char* txt, graphics_t* gr, char do_draw)
{
	sfVector2i imouse = sfMouse_getPosition((sfWindow*) gr->render);
	sfVector2f mouse = {imouse.x, imouse.y};

	sfSprite* sprite = NULL;
	if (sprite == NULL)
	{
		int id = graphics_spriteForImg(gr, "menubutton.png");
		sprite = gr->sprites[id];
	}

	sfFloatRect rect = sfSprite_getGlobalBounds(sprite);
	sfVector2f pos = {x-rect.width/2, y-rect.height/2};
	sfSprite_setPosition(sprite, pos);

	char is_in = sfSprite_contains(sprite, mouse);
	sfColor color = is_in ? sfWhite : (sfColor){255,255,255,127};
	sfSprite_setColor(sprite, color);

	if (do_draw)
		sfRenderWindow_drawSprite(gr->render, sprite, NULL);
	else
		return is_in;

	sfText* text = NULL;
	if (text == NULL)
	{
		text = sfText_create();
		sfText_setFont(text, gr->font);
		sfText_setCharacterSize(text, 25);
	}

	sfText_setUTF8(text, txt);
	rect = sfText_getLocalBounds(text);
	pos = (sfVector2f){x-rect.width/2-rect.left, y-rect.height/2-rect.top};
	sfText_setPosition(text, pos);
	sfText_setOrigin(text, (sfVector2f){0,0});
	sfRenderWindow_drawText(gr->render, text, NULL);

	return 0;
}
static char draw_buttons(graphics_t* gr, char do_draw)
{
	sfVector2u size = sfRenderWindow_getSize(gr->render);
	float x = size.x / 2;
	float y = size.y / 2 - 100;
	const char* labels[] =
	{
		"Jouer !",
		"Configurer",
		"Quitter",
	};

	for (size_t i = 0; i < sizeof(labels)/sizeof(labels[0]); i++)
	{
		if (draw_button(x, y, labels[i], gr, do_draw))
			return i;

		y += 50;
	}

	return -1;
}
void menu(settings_t* s)
{
	graphics_t* gr = graphics_init();

	(void) s;

	int id = graphics_spriteForImg(gr, "menu.png");
	sfSprite* illustration = gr->sprites[id];

	sfRenderWindow* render = gr->render;
	while (sfRenderWindow_isOpen(render))
	{
		sfEvent event;
		while (sfRenderWindow_pollEvent(render, &event))
		{
			if (event.type == sfEvtClosed)
			{
				graphics_exit(gr);
				exit(0);
			}
			else if (event.type == sfEvtResized)
			{
				sfFloatRect rect = {0,0,event.size.width,event.size.height};
				sfView* view = sfView_createFromRect(rect);
				sfRenderWindow_setView(render, view);
				sfView_destroy(view);
			}
			else if (event.type == sfEvtKeyReleased)
			{
				if (event.key.code == sfKeyEscape)
				{
					graphics_exit(gr);
					exit(0);
				}
				else if (event.key.code == sfKeyReturn)
					return;
			}
			else if (event.type == sfEvtMouseButtonReleased)
			{
				int i = draw_buttons(gr, 0);
				if (i == 0)
				{
					game_t game;
					game_init(&game, s, gr);
					game_loop(&game);
					game_exit(&game);
				}
				else if (i == 1)
				{
				}
				else if (i == 2)
				{
					graphics_exit(gr);
					exit(0);
				}
			}
		}

		sfRenderWindow_clear(render, sfBlack);

		sfVector2u size  = sfRenderWindow_getSize(render);
		sfFloatRect rect = sfSprite_getGlobalBounds(illustration);
		sfVector2f pos = {size.x/2 - rect.width/2, size.y/2-rect.height/2};
		sfSprite_setPosition(illustration, pos);
		sfRenderWindow_drawSprite(render, illustration, NULL);

		draw_buttons(gr, 1);

		graphics_drawCursor(gr, 0);
		sfRenderWindow_display(render);
	}

	graphics_exit(gr);
}
